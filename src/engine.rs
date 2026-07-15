use crate::commands::PdfCommand;
use crate::pdf_engine::{DocumentStore, SharedRenderCache, create_render_cache};
use pdfium_render::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct EngineState {
    pub cmd_tx: mpsc::Sender<PdfCommand>,
}

/// Re-open a document from its remembered path if it isn't currently loaded.
fn reload_if_needed(
    store: &mut DocumentStore,
    paths: &Arc<RwLock<HashMap<crate::models::DocumentId, String>>>,
    doc_id: crate::models::DocumentId,
) {
    if !store.has_document(doc_id) {
        if let Ok(guard) = paths.read() {
            if let Some(path) = guard.get(&doc_id).cloned() {
                let _ = store.open_document(&path, doc_id);
            }
        }
    }
}

/// Resolve the directory containing the running executable, with the Windows
/// UNC/extended-path prefix (`\\?\`) stripped. Used for PDFium/library loading.
fn exe_dir_raw() -> Option<String> {
    std::env::current_exe().ok()?.parent().map(|dir| {
        let mut dir_str = dir.to_string_lossy().into_owned();
        if let Some(stripped) = dir_str.strip_prefix(r"\\?\") {
            dir_str = stripped.to_string();
        }
        dir_str
    })
}

#[must_use]
pub fn spawn_engine_thread(cache_size: u64, max_memory_mb: u64) -> EngineState {
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<PdfCommand>(128);

    let render_cache: SharedRenderCache = create_render_cache(cache_size, max_memory_mb);

    // Shared paths mapping between all concurrent threads
    let shared_paths = Arc::new(RwLock::new(HashMap::new()));

    // MPMC channel for distributing tasks across the thread pool
    let (worker_tx, worker_rx) = crossbeam_channel::bounded::<PdfCommand>(256);

    // Forward Tokio commands into the crossbeam MPMC channel
    tokio::spawn(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            let _ = worker_tx.send(cmd);
        }
    });

    // ── PDFium initialization ──────────────────────────────────────────────────
    // On Windows, explicitly add the executable's directory to the DLL search path.
    // This resolves LoadLibrary failures in protected system folders like C:\Program Files\.
    #[cfg(windows)]
    {
        if let Some(dir_str) = exe_dir_raw() {
            let mut path_w: Vec<u16> = dir_str.encode_utf16().collect();
            path_w.push(0); // Null terminator
            unsafe {
                #[link(name = "kernel32")]
                unsafe extern "system" {
                    fn SetDllDirectoryW(lpPathName: *const u16) -> i32;
                }
                let _ = SetDllDirectoryW(path_w.as_ptr());
            }
        }
    }

    // The pdfium-render crate registers bindings globally (one per process).
    // Calling bind_to_library more than once returns PdfiumLibraryBindingsAlreadyInitialized.
    // We therefore initialize exactly once here, on the calling thread, and share
    // the resulting Pdfium instance across all worker threads via Arc.
    // The `thread_safe` feature in Cargo.toml makes Pdfium: Send + Sync.
    let pdfium: Arc<Pdfium> = {
        // 1. Try the directory that contains the running executable (production installs,
        //    Desktop/Start Menu shortcuts, file-association launches — all cases where the
        //    process working-directory differs from the install folder).
        let exe_dir_bindings = exe_dir_raw().and_then(|mut dir_str| {
            // Append a trailing separator so bind_to_library resolves the file.
            if !dir_str.ends_with('/') && !dir_str.ends_with('\\') {
                #[cfg(windows)]
                dir_str.push('\\');
                #[cfg(not(windows))]
                dir_str.push('/');
            }
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(&dir_str)).ok()
        });

        let bindings = if let Some(b) = exe_dir_bindings {
            tracing::info!("PDFium bound from executable directory.");
            b
        } else if let Ok(b) = Pdfium::bind_to_system_library() {
            // 2. System-wide library search paths.
            tracing::info!("PDFium bound from system library.");
            b
        } else {
            // 3. Current working directory fallback (dev runs / cargo run).
            tracing::warn!(
                "PDFium not found in executable directory or system paths. Trying './'..."
            );
            match Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./")) {
                Ok(b) => {
                    tracing::info!("PDFium bound from current working directory.");
                    b
                }
                Err(e) => {
                    tracing::error!("CRITICAL: Could not find or load pdfium: {e}");
                    // Diagnostic logging to AppData
                    if let Some(proj_dirs) =
                        directories::ProjectDirs::from("", "SV-stark", "PDFbull")
                    {
                        let log_path = proj_dirs.config_dir().join("engine_error.log");
                        let _ = std::fs::create_dir_all(proj_dirs.config_dir());
                        let _ = std::fs::write(
                            &log_path,
                            format!("Failed to load PDFium: {}\nDetails: {:?}", e, e),
                        );
                    }
                    return EngineState { cmd_tx };
                }
            }
        };

        Arc::new(Pdfium::new(bindings))
    };

    // Spawn exactly 1 background worker thread to process all PDF operations sequentially.
    // PDFium is serialized internally by a global mutex, so there is no parallel rendering benefit
    // from multiple threads, but a single thread completely eliminates stale thread-local state desyncs
    // and redundant file-loading memory bloat.
    let rx = worker_rx.clone();
    let cache = render_cache.clone();
    let paths = shared_paths.clone();
    // Arc clone: cheap reference-count increment, no re-initialization.
    let pdfium = pdfium.clone();

    std::thread::spawn(move || {
        let mut store = DocumentStore::new(&pdfium, cache);

        while let Ok(cmd) = rx.recv() {
            match cmd {
                PdfCommand::Open(path, doc_id, tx) => {
                    let res = store.open_document(&path, doc_id);
                    if res.is_ok() {
                        if let Ok(mut guard) = paths.write() {
                            guard.insert(doc_id, path);
                        }
                    }
                    let _ = tx.send(res);
                }
                PdfCommand::Render(doc_id, page_num, options, tx) => {
                    reload_if_needed(&mut store, &paths, doc_id);
                    let res = store.render_page(doc_id, page_num, options);
                    let _ = tx.send(res);
                }
                PdfCommand::RenderThumbnail(doc_id, page_num, scale, tx) => {
                    reload_if_needed(&mut store, &paths, doc_id);
                    let options = crate::pdf_engine::RenderOptions {
                        scale,
                        rotation: 0,
                        filter: crate::pdf_engine::RenderFilter::None,
                        auto_crop: false,
                        quality: crate::pdf_engine::RenderQuality::Low,
                    };
                    let res = store.render_thumbnail(doc_id, page_num, options);
                    let _ = tx.send(res);
                }
                PdfCommand::Close(doc_id) => {
                    store.close_document(doc_id);
                    if let Ok(mut guard) = paths.write() {
                        guard.remove(&doc_id);
                    }
                }
                PdfCommand::ExtractText(doc_id, page_num, tx) => {
                    reload_if_needed(&mut store, &paths, doc_id);
                    let res = store.extract_text(doc_id, page_num);
                    let _ = tx.send(res);
                }
                PdfCommand::Search(doc_id, query, tx) => {
                    reload_if_needed(&mut store, &paths, doc_id);
                    let res = store.search(doc_id, &query);
                    let _ = tx.send(res);
                }
                PdfCommand::GetTextItems(doc_id, page_num, tx) => {
                    reload_if_needed(&mut store, &paths, doc_id);
                    let res = store.extract_text_items(doc_id, page_num);
                    let _ = tx.send(res);
                }
                PdfCommand::LoadDocumentMeta(doc_id, tx) => {
                    reload_if_needed(&mut store, &paths, doc_id);
                    let res = store.load_document_meta(doc_id);
                    let _ = tx.send(res);
                }
                PdfCommand::SaveAnnotations(doc_id, annotations, tx) => {
                    reload_if_needed(&mut store, &paths, doc_id);
                    let res = store.save_annotations(doc_id, &annotations, None);
                    let _ = tx.send(res);
                }
                PdfCommand::ExportImage(doc_id, page_num, scale, tx) => {
                    reload_if_needed(&mut store, &paths, doc_id);
                    let res = store.export_page_as_image(doc_id, page_num, scale);
                    let _ = tx.send(res);
                }
                PdfCommand::ExportImages(doc_id, pages, scale, out_dir, tx) => {
                    reload_if_needed(&mut store, &paths, doc_id);
                    let out_path = std::path::Path::new(&out_dir);
                    if !out_path.is_dir() {
                        let _ = tx.send(Err(crate::models::PdfError::IoError(
                            "Output directory does not exist".into(),
                        )));
                        continue;
                    }
                    let mut output_paths = Vec::new();
                    for page_num in pages {
                        let safe_name = format!("page_{page_num}.png");
                        let out_file = out_path.join(&safe_name);
                        if let Ok(buf) = store.export_page_as_image(doc_id, page_num, scale) {
                            let optimized =
                                oxipng::optimize_from_memory(&buf, &oxipng::Options::default())
                                    .unwrap_or(buf);
                            if std::fs::write(&out_file, optimized).is_ok()
                                && let Some(path_str) = out_file.to_str()
                            {
                                output_paths.push(path_str.to_string());
                            }
                        }
                    }
                    let _ = tx.send(Ok(output_paths));
                }
                PdfCommand::ExportPdf(doc_id, path, annotations, tx) => {
                    reload_if_needed(&mut store, &paths, doc_id);
                    let res = store.save_annotations(doc_id, &annotations, Some(path));
                    let _ = tx.send(res);
                }
                PdfCommand::Merge(paths_list, out, tx) => {
                    let res = store.merge_documents(paths_list, out);
                    let _ = tx.send(res);
                }
                PdfCommand::Split(path, pages, out, tx) => {
                    let res = store.split_pdf(&path, pages, out);
                    let _ = tx.send(res);
                }
                PdfCommand::GetFormFields(path, tx) => {
                    let res = store.get_form_fields(&path);
                    let _ = tx.send(res);
                }
                PdfCommand::FillForm(path, fields, out, tx) => {
                    let res = store.fill_form(&path, fields, out);
                    let _ = tx.send(res);
                }
                PdfCommand::PrintPdf(path, printer_name, tx) => {
                    let res = crate::pdf_engine::DocumentStore::print_document(
                        &path,
                        printer_name.as_deref(),
                    );
                    let _ = tx.send(res);
                }
                PdfCommand::ListPrinters(tx) => {
                    let _ = tx.send(crate::pdf_engine::DocumentStore::list_printers());
                }
                PdfCommand::AddWatermark(input, text, output, tx) => {
                    let res =
                        crate::pdf_engine::DocumentStore::add_watermark(&input, &text, &output);
                    let _ = tx.send(res);
                }
                _ => {}
            }
        }
    });

    EngineState { cmd_tx }
}
