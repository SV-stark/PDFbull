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
                if let Err(e) = store.open_document(&path, None, doc_id) {
                    tracing::error!(
                        "Failed to reload document {doc_id:?} from path '{path}': {e:?}"
                    );
                }
            }
        }
    }
}

fn catch_worker_panic<F, T>(cmd_name: &str, f: F) -> Result<T, crate::models::PdfError>
where
    F: FnOnce() -> Result<T, crate::models::PdfError> + std::panic::UnwindSafe,
{
    match std::panic::catch_unwind(f) {
        Ok(res) => res,
        Err(err) => {
            let panic_msg = if let Some(s) = err.downcast_ref::<&str>() {
                *s
            } else if let Some(s) = err.downcast_ref::<String>() {
                s.as_str()
            } else {
                "unknown panic"
            };
            tracing::error!("Engine worker panicked during {cmd_name}: {panic_msg}");
            Err(crate::models::PdfError::EngineDied)
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

    let num_workers = std::thread::available_parallelism()
        .map(std::num::NonZero::get)
        .unwrap_or(4)
        .clamp(2, 4);

    // Forward Tokio mpsc commands into the crossbeam MPMC channel.
    // iced uses the `tokio` feature so a full multi-thread runtime is always
    // available here; tokio::spawn is safe and keeps the forwarder alive for
    // the lifetime of the iced application.
    tokio::spawn(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            if let PdfCommand::Close(doc_id) = cmd {
                for _ in 0..num_workers {
                    let _ = worker_tx.send(PdfCommand::Close(doc_id));
                }
            } else {
                let _ = worker_tx.send(cmd);
            }
        }
        tracing::debug!("Engine forwarder task exited (cmd_tx dropped)");
    });

    for _ in 0..num_workers {
        let rx = worker_rx.clone();
        let cache = render_cache.clone();
        let paths = shared_paths.clone();

        std::thread::spawn(move || {
            let mut store = DocumentStore::new(cache);

            while let Ok(cmd) = rx.recv() {
                match cmd {
                    PdfCommand::Open(path, password, doc_id, tx) => {
                        tracing::info!("Engine worker: opening {:?}", path);
                        let mut store_ref = std::panic::AssertUnwindSafe(&mut store);
                        let path_clone = path.clone();
                        let pass_clone = password.clone();
                        let res = catch_worker_panic("open", move || {
                            store_ref.open_document(&path_clone, pass_clone.as_deref(), doc_id)
                        });

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
                        let res = catch_worker_panic("render", move || {
                            store_ref.render_page(doc_id, page_num, options)
                        });

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
                        let mut store_ref = std::panic::AssertUnwindSafe(&mut store);
                        let res = catch_worker_panic("render_thumbnail", move || {
                            store_ref.render_thumbnail(doc_id, page_num, options)
                        });
                        let _ = tx.send(res);
                    }
                    PdfCommand::Close(doc_id) => {
                        let mut store_ref = std::panic::AssertUnwindSafe(&mut store);
                        let _ = std::panic::catch_unwind(move || {
                            store_ref.close_document(doc_id);
                        });
                        if let Ok(mut guard) = paths.write() {
                            guard.remove(&doc_id);
                        }
                    }
                    PdfCommand::ExtractText(doc_id, page_num, tx) => {
                        reload_if_needed(&mut store, &paths, doc_id);
                        let store_ref = std::panic::AssertUnwindSafe(&store);
                        let res = catch_worker_panic("extract_text", move || {
                            store_ref.extract_text(doc_id, page_num)
                        });
                        let _ = tx.send(res);
                    }
                    PdfCommand::Search(doc_id, query, tx) => {
                        reload_if_needed(&mut store, &paths, doc_id);
                        let store_ref = std::panic::AssertUnwindSafe(&store);
                        let res =
                            catch_worker_panic("search", move || store_ref.search(doc_id, &query));
                        let _ = tx.send(res);
                    }
                    PdfCommand::GetTextItems(doc_id, page_num, tx) => {
                        reload_if_needed(&mut store, &paths, doc_id);
                        let store_ref = std::panic::AssertUnwindSafe(&store);
                        let res = catch_worker_panic("get_text_items", move || {
                            store_ref.extract_text_items(doc_id, page_num)
                        });
                        let _ = tx.send(res);
                    }
                    PdfCommand::LoadDocumentMeta(doc_id, tx) => {
                        reload_if_needed(&mut store, &paths, doc_id);
                        let mut store_ref = std::panic::AssertUnwindSafe(&mut store);
                        let res = catch_worker_panic("load_document_meta", move || {
                            store_ref.load_document_meta(doc_id)
                        });
                        let _ = tx.send(res);
                    }
                    PdfCommand::SaveAnnotations(doc_id, annotations, tx) => {
                        reload_if_needed(&mut store, &paths, doc_id);
                        let mut store_ref = std::panic::AssertUnwindSafe(&mut store);
                        let res = catch_worker_panic("save_annotations", move || {
                            store_ref.save_annotations(doc_id, &annotations, None)
                        });
                        let _ = tx.send(res);
                    }
                    PdfCommand::ExportImage(doc_id, page_num, scale, tx) => {
                        reload_if_needed(&mut store, &paths, doc_id);
                        let store_ref = std::panic::AssertUnwindSafe(&store);
                        let res = catch_worker_panic("export_image", move || {
                            store_ref.export_page_as_image(doc_id, page_num, scale)
                        });
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
                        let store_ref = std::panic::AssertUnwindSafe(&store);
                        let res = catch_worker_panic("export_images", move || {
                            let mut output_paths = Vec::new();
                            for page_num in pages {
                                let safe_name = format!("page_{page_num}.png");
                                let out_file = out_path.join(&safe_name);
                                if let Ok(buf) =
                                    store_ref.export_page_as_image(doc_id, page_num, scale)
                                {
                                    let optimized = oxipng::optimize_from_memory(
                                        &buf,
                                        &oxipng::Options::default(),
                                    )
                                    .unwrap_or(buf);
                                    if std::fs::write(&out_file, optimized).is_ok()
                                        && let Some(path_str) = out_file.to_str()
                                    {
                                        output_paths.push(path_str.to_string());
                                    }
                                }
                            }
                            Ok(output_paths)
                        });
                        let _ = tx.send(res);
                    }
                    PdfCommand::ExportPdf(doc_id, path, annotations, tx) => {
                        reload_if_needed(&mut store, &paths, doc_id);
                        let mut store_ref = std::panic::AssertUnwindSafe(&mut store);
                        let res = catch_worker_panic("export_pdf", move || {
                            store_ref.save_annotations(doc_id, &annotations, Some(path))
                        });
                        let _ = tx.send(res);
                    }
                    PdfCommand::Merge(paths_list, out, tx) => {
                        let store_ref = std::panic::AssertUnwindSafe(&store);
                        let res = catch_worker_panic("merge", move || {
                            store_ref.merge_documents(paths_list, out)
                        });
                        let _ = tx.send(res);
                    }
                    PdfCommand::Split(path, pages, out, tx) => {
                        let store_ref = std::panic::AssertUnwindSafe(&store);
                        let res = catch_worker_panic("split", move || {
                            store_ref.split_pdf(&path, pages, out)
                        });
                        let _ = tx.send(res);
                    }
                    PdfCommand::GetFormFields(path, tx) => {
                        let mut store_ref = std::panic::AssertUnwindSafe(&mut store);
                        let res = catch_worker_panic("get_form_fields", move || {
                            store_ref.get_form_fields(&path)
                        });
                        let _ = tx.send(res);
                    }
                    PdfCommand::FillForm(path, fields, out, tx) => {
                        let mut store_ref = std::panic::AssertUnwindSafe(&mut store);
                        let res = catch_worker_panic("fill_form", move || {
                            store_ref.fill_form(&path, fields, out)
                        });
                        let _ = tx.send(res);
                    }
                    PdfCommand::PrintPdf(path, printer_name, tx) => {
                        let res = catch_worker_panic("print_pdf", move || {
                            crate::pdf_engine::DocumentStore::print_document(
                                &path,
                                printer_name.as_deref(),
                            )
                        });
                        let _ = tx.send(res);
                    }
                    PdfCommand::ListPrinters(tx) => {
                        let res = catch_worker_panic("list_printers", move || {
                            crate::pdf_engine::DocumentStore::list_printers()
                        });
                        let _ = tx.send(res);
                    }
                    PdfCommand::AddWatermark(input, text, output, tx) => {
                        let res = catch_worker_panic("add_watermark", move || {
                            crate::pdf_engine::DocumentStore::add_watermark(&input, &text, &output)
                        });
                        let _ = tx.send(res);
                    }
                    PdfCommand::AddHeaderFooter(input, header, footer, output, tx) => {
                        let res = catch_worker_panic("add_header_footer", move || {
                            crate::pdf_engine::DocumentStore::add_header_footer(
                                &input, &header, &footer, &output,
                            )
                        });
                        let _ = tx.send(res);
                    }
                    PdfCommand::Optimize(input, output, tx) => {
                        let store_ref = std::panic::AssertUnwindSafe(&store);
                        let res = catch_worker_panic("optimize", move || {
                            store_ref.optimize_pdf(&input, &output)
                        });
                        let _ = tx.send(res);
                    }
                    PdfCommand::ReorderPages(input, page_order, output, tx) => {
                        let store_ref = std::panic::AssertUnwindSafe(&store);
                        let res = catch_worker_panic("reorder_pages", move || {
                            store_ref.reorder_pages(&input, &page_order, &output)
                        });
                        let _ = tx.send(res);
                    }
                    PdfCommand::LoadAnnotations(doc_id, path, tx) => {
                        let _ = doc_id;
                        let store_ref = std::panic::AssertUnwindSafe(&store);
                        let res = catch_worker_panic("load_annotations", move || {
                            store_ref.load_annotations(&path)
                        });
                        let _ = tx.send(res);
                    }
                    PdfCommand::ToggleLayer(doc_id, object_id, visible) => {
                        reload_if_needed(&mut store, &paths, doc_id);
                        let mut store_ref = std::panic::AssertUnwindSafe(&mut store);
                        let _ = std::panic::catch_unwind(move || {
                            store_ref.toggle_layer(doc_id, object_id, visible);
                        });
                    }
                    PdfCommand::GetAttachmentBytes(doc_id, object_id, tx) => {
                        reload_if_needed(&mut store, &paths, doc_id);
                        let store_ref = std::panic::AssertUnwindSafe(&store);
                        let res = catch_worker_panic("get_attachment_bytes", move || {
                            store_ref.get_attachment_bytes(doc_id, object_id)
                        });
                        let _ = tx.send(res);
                    }
                    PdfCommand::DetectTables(doc_id, page_num, tx) => {
                        reload_if_needed(&mut store, &paths, doc_id);
                        let store_ref = std::panic::AssertUnwindSafe(&store);
                        let res = catch_worker_panic("detect_tables", move || {
                            store_ref.detect_tables_on_page(doc_id, page_num)
                        });
                        let _ = tx.send(res);
                    }
                    PdfCommand::EncryptPdf(input, output, user, owner, algo, tx) => {
                        let store_ref = std::panic::AssertUnwindSafe(&store);
                        let res = catch_worker_panic("encrypt_pdf", move || {
                            store_ref.encrypt_pdf(&input, &output, &user, &owner, &algo)
                        });
                        let _ = tx.send(res);
                    }
                    PdfCommand::LinearizePdf(input, output, tx) => {
                        let store_ref = std::panic::AssertUnwindSafe(&store);
                        let res = catch_worker_panic("linearize_pdf", move || {
                            store_ref.linearize_pdf(&input, &output)
                        });
                        let _ = tx.send(res);
                    }
                    PdfCommand::ValidatePdfA(path, profile, tx) => {
                        let store_ref = std::panic::AssertUnwindSafe(&store);
                        let res = catch_worker_panic("validate_pdfa", move || {
                            store_ref.validate_pdfa(&path, &profile)
                        });
                        let _ = tx.send(res);
                    }
                    PdfCommand::VerifySignatureTrust(doc_id, anchors, tx) => {
                        reload_if_needed(&mut store, &paths, doc_id);
                        let store_ref = std::panic::AssertUnwindSafe(&store);
                        let res = catch_worker_panic("verify_signature_trust", move || {
                            store_ref.verify_signature_trust(doc_id, &anchors)
                        });
                        let _ = tx.send(res);
                    }
                    PdfCommand::ConvertPdf(doc_id, mode, format, tx) => {
                        reload_if_needed(&mut store, &paths, doc_id);
                        let store_ref = std::panic::AssertUnwindSafe(&store);
                        let res = catch_worker_panic("convert_pdf", move || {
                            store_ref.convert_pdf_doc_by_id(doc_id, &mode, &format)
                        });
                        let _ = tx.send(res);
                    }
                    PdfCommand::CreateBlankDocument(output_path, tx) => {
                        let res = catch_worker_panic("create_blank_document", move || {
                            crate::pdf_engine::DocumentStore::create_blank_document(&output_path)
                        });
                        let _ = tx.send(res);
                    }
                    PdfCommand::SignDocumentWithCert(doc_id, cert_path, output_path, tx) => {
                        reload_if_needed(&mut store, &paths, doc_id);
                        let store_ref = std::panic::AssertUnwindSafe(&store);
                        let res = catch_worker_panic("sign_with_certificate", move || {
                            store_ref.sign_with_certificate(doc_id, &cert_path, &output_path)
                        });
                        let _ = tx.send(res);
                    }
                    PdfCommand::ApplyStamp(doc_id, page_num, label, output_path, tx) => {
                        reload_if_needed(&mut store, &paths, doc_id);
                        let store_ref = std::panic::AssertUnwindSafe(&store);
                        let res = catch_worker_panic("apply_stamp", move || {
                            store_ref.apply_stamp(doc_id, page_num, &label, &output_path)
                        });
                        let _ = tx.send(res);
                    }
                    PdfCommand::OcrPage(doc_id, page_num, script, tx) => {
                        reload_if_needed(&mut store, &paths, doc_id);
                        let store_ref = std::panic::AssertUnwindSafe(&store);
                        let res = catch_worker_panic("ocr_page", move || {
                            store_ref.ocr_page(doc_id, page_num, script)
                        });
                        let _ = tx.send(res);
                    }
                }
            }
        });
    }

    EngineState { cmd_tx }
}
