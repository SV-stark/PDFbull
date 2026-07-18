use crate::commands::PdfCommand;
use crate::pdf_engine::{DocumentStore, SharedRenderCache, create_render_cache};
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

#[must_use]
pub fn spawn_engine_thread(cache_size: u64, max_memory_mb: u64) -> EngineState {
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<PdfCommand>(128);

    let render_cache: SharedRenderCache = create_render_cache(cache_size, max_memory_mb);

    // Shared paths mapping between all concurrent threads
    let shared_paths = Arc::new(RwLock::new(HashMap::new()));

    // MPMC channel for distributing tasks across the thread pool
    let (worker_tx, worker_rx) = crossbeam_channel::bounded::<PdfCommand>(256);

    // Forward Tokio mpsc commands into the crossbeam MPMC channel.
    // iced uses the `tokio` feature so a full multi-thread runtime is always
    // available here; tokio::spawn is safe and keeps the forwarder alive for
    // the lifetime of the iced application.
    tokio::spawn(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            let _ = worker_tx.send(cmd);
        }
        tracing::debug!("Engine forwarder task exited (cmd_tx dropped)");
    });

    let num_workers = std::thread::available_parallelism()
        .map(std::num::NonZero::get)
        .unwrap_or(4)
        .clamp(2, 8);

    for _ in 0..num_workers {
        let rx = worker_rx.clone();
        let cache = render_cache.clone();
        let paths = shared_paths.clone();

        std::thread::spawn(move || {
            let mut store = DocumentStore::new(cache);

            while let Ok(cmd) = rx.recv() {
                match cmd {
                    PdfCommand::Open(path, doc_id, tx) => {
                        tracing::info!("Engine worker: opening {:?}", path);
                        let mut store_ref = std::panic::AssertUnwindSafe(&mut store);
                        let path_clone = path.clone();
                        let result = std::panic::catch_unwind(move || {
                            store_ref.open_document(&path_clone, doc_id)
                        });

                        let res = match result {
                            Ok(res) => res,
                            Err(err) => {
                                let panic_msg = if let Some(s) = err.downcast_ref::<&str>() {
                                    *s
                                } else if let Some(s) = err.downcast_ref::<String>() {
                                    s.as_str()
                                } else {
                                    "unknown panic"
                                };
                                tracing::error!(
                                    "Engine worker panicked during open: {}",
                                    panic_msg
                                );
                                Err(crate::models::PdfError::EngineDied)
                            }
                        };

                        if res.is_ok() {
                            if let Ok(mut guard) = paths.write() {
                                guard.insert(doc_id, path);
                            }
                        } else {
                            tracing::error!("Engine worker: open failed: {:?}", res);
                        }
                        let _ = tx.send(res);
                    }
                    PdfCommand::Render(doc_id, page_num, options, tx) => {
                        tracing::debug!("Engine worker: render page {} for {:?}", page_num, doc_id);
                        reload_if_needed(&mut store, &paths, doc_id);

                        let mut store_ref = std::panic::AssertUnwindSafe(&mut store);
                        let result = std::panic::catch_unwind(move || {
                            store_ref.render_page(doc_id, page_num, options)
                        });

                        let res = match result {
                            Ok(res) => res,
                            Err(err) => {
                                let panic_msg = if let Some(s) = err.downcast_ref::<&str>() {
                                    *s
                                } else if let Some(s) = err.downcast_ref::<String>() {
                                    s.as_str()
                                } else {
                                    "unknown panic"
                                };
                                tracing::error!(
                                    "Engine worker panicked during render page {}: {}",
                                    page_num,
                                    panic_msg
                                );
                                Err(crate::models::PdfError::EngineDied)
                            }
                        };

                        if res.is_err() {
                            tracing::error!(
                                "Engine worker: render page {} failed: {:?}",
                                page_num,
                                res
                            );
                        }
                        let _ = tx.send(res);
                    }
                    PdfCommand::RenderThumbnail(doc_id, page_num, scale, rotation, tx) => {
                        reload_if_needed(&mut store, &paths, doc_id);
                        let options = crate::pdf_engine::RenderOptions {
                            scale,
                            rotation,
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
                    PdfCommand::Optimize(input, output, tx) => {
                        let res = store.optimize_pdf(&input, &output);
                        let _ = tx.send(res);
                    }
                    PdfCommand::ReorderPages(input, page_order, output, tx) => {
                        let res = store.reorder_pages(&input, &page_order, &output);
                        let _ = tx.send(res);
                    }
                    _ => {}
                }
            }
        });
    }

    EngineState { cmd_tx }
}
