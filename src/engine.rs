use crate::commands::PdfCommand;
use crate::pdf_engine::{create_render_cache, DocumentStore, SharedRenderCache};
use pdfium_render::prelude::*;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct EngineState {
    pub cmd_tx: mpsc::UnboundedSender<PdfCommand>,
    pub active_workers: Arc<std::sync::atomic::AtomicUsize>,
}

#[must_use]
pub fn spawn_engine_thread(cache_size: u64, max_memory_mb: u64) -> EngineState {
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<PdfCommand>();
    let active_workers = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    let render_cache: SharedRenderCache = create_render_cache(cache_size, max_memory_mb);

    std::thread::spawn(move || {
        let pdfium = if let Ok(p) = Pdfium::bind_to_system_library() {
            Pdfium::new(p)
        } else {
            tracing::error!("Failed to bind to Pdfium system library. Attempting local search...");
            match Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./")) {
                Ok(p) => Pdfium::new(p),
                Err(e) => {
                    tracing::error!("CRITICAL: Could not find Pdfium: {e}");
                    return;
                }
            }
        };

        let mut store = match DocumentStore::new(&pdfium, render_cache.clone()) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to initialize DocumentStore: {e}");
                return;
            }
        };

        while let Some(cmd) = cmd_rx.blocking_recv() {
            match cmd {
                PdfCommand::Open(path, doc_id, tx) => {
                    let res = store.open_document(&path, doc_id);
                    let _ = tx.send(res);
                }
                PdfCommand::Render(doc_id, page_num, options, tx) => {
                    let res = store.render_page(doc_id, page_num, options);
                    let _ = tx.send(res);
                }
                PdfCommand::RenderThumbnail(doc_id, page_num, scale, tx) => {
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
                }
                PdfCommand::ExtractText(doc_id, page_num, tx) => {
                    let res = store.extract_text(doc_id, page_num);
                    let _ = tx.send(res);
                }
                PdfCommand::Search(doc_id, query, tx) => {
                    let res = store.search(doc_id, &query);
                    let _ = tx.send(res);
                }
                PdfCommand::SaveAnnotations(doc_id, annotations, tx) => {
                    let res = store.save_annotations(doc_id, &annotations, None);
                    let _ = tx.send(res);
                }
                PdfCommand::ExportImage(doc_id, page_num, scale, tx) => {
                    let res = store.export_page_as_image(doc_id, page_num, scale);
                    let _ = tx.send(res);
                }
                PdfCommand::ExportImages(doc_id, pages, scale, out_dir, tx) => {
                    let mut paths = Vec::new();
                    for page_num in pages {
                        let out_path = format!("{out_dir}/page_{page_num}.png");
                        if let Ok(buf) = store.export_page_as_image(doc_id, page_num, scale) {
                            let optimized =
                                oxipng::optimize_from_memory(&buf, &oxipng::Options::default())
                                    .unwrap_or(buf);
                            if std::fs::write(&out_path, optimized).is_ok() {
                                paths.push(out_path);
                            }
                        }
                    }
                    let _ = tx.send(Ok(paths));
                }
                PdfCommand::ExportPdf(doc_id, path, annotations, tx) => {
                    let res = store.save_annotations(doc_id, &annotations, Some(path));
                    let _ = tx.send(res);
                }
                PdfCommand::Merge(paths, out, tx) => {
                    let res = store.merge_documents(paths, out);
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
                PdfCommand::PrintPdf(path, tx) => {
                    let res = crate::pdf_engine::DocumentStore::print_document(&path);
                    let _ = tx.send(res);
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

    EngineState {
        cmd_tx,
        active_workers,
    }
}
