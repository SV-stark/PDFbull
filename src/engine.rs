use crate::commands::PdfCommand;
use crate::models::{next_doc_id, DocumentId};
use crate::pdf_engine::{create_render_cache, DocumentStore, SharedRenderCache};
use pdfium_render::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

#[derive(Debug, Clone)]
pub struct EngineState {
    pub cmd_tx: crossbeam_channel::Sender<PdfCommand>,
    active_workers: Arc<AtomicUsize>,
}

impl EngineState {
    pub fn worker_count(&self) -> usize {
        self.active_workers.load(Ordering::SeqCst)
    }
}

fn spawn_document_worker(
    doc_id: DocumentId,
    path: String,
    cmd_rx: crossbeam_channel::Receiver<PdfCommand>,
    cache: SharedRenderCache,
    active_workers: Arc<AtomicUsize>,
    pdfium: Arc<Pdfium>,
) {
    active_workers.fetch_add(1, Ordering::SeqCst);

    thread::spawn(move || {
        let mut store = match DocumentStore::new(&*pdfium, cache) {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to initialize DocumentStore: {}", e);
                active_workers.fetch_sub(1, Ordering::SeqCst);
                return;
            }
        };

        // Pre-open the document in this thread
        // This ensures the document is loaded EXACTLY once for this thread's lifetime
        if let Err(e) = store.open_document(&path) {
            log::error!("Failed to open document in worker thread: {}", e);
            // We still need to listen for the initial Open command to send the error back
        }

        while let Ok(cmd) = cmd_rx.recv() {
            match cmd {
                PdfCommand::Open(p, resp) => {
                    // Since we already opened it above, we just need to send the shared state back
                    match store.open_document(&p) {
                        Ok((path_str, count, heights, width)) => {
                            let outline = store.get_outline(&path_str);
                            let _ = resp.send(Ok((doc_id, count, heights, width, outline)));
                        }
                        Err(e) => {
                            let _ = resp.send(Err(e));
                        }
                    }
                }
                PdfCommand::Render(_, page, options, resp) => {
                    match store.render_page(&path, page, options) {
                        Ok((w, h, data)) => {
                            let _ = resp.send(Ok((w, h, data)));
                        }
                        Err(e) => {
                            let _ = resp.send(Err(e));
                        }
                    }
                }
                PdfCommand::RenderThumbnail(_, page, zoom, resp) => {
                    match store.render_page(
                        &path,
                        page,
                        crate::pdf_engine::RenderOptions {
                            scale: zoom,
                            rotation: 0,
                            filter: crate::pdf_engine::RenderFilter::None,
                            auto_crop: false,
                            quality: crate::pdf_engine::RenderQuality::Low,
                        },
                    ) {
                        Ok((w, h, data)) => {
                            let _ = resp.send(Ok((w, h, data)));
                        }
                        Err(e) => {
                            let _ = resp.send(Err(e));
                        }
                    }
                }
                PdfCommand::ExtractText(_, page, resp) => match store.extract_text(&path, page) {
                    Ok(text) => {
                        let _ = resp.send(Ok(text));
                    }
                    Err(e) => {
                        let _ = resp.send(Err(e));
                    }
                },
                PdfCommand::ExportImage(_, page, zoom, output_path, resp) => {
                    match store.export_page_as_image(&path, page, zoom, &output_path) {
                        Ok(()) => {
                            let _ = resp.send(Ok(()));
                        }
                        Err(e) => {
                            let _ = resp.send(Err(e));
                        }
                    }
                }
                PdfCommand::ExportImages(_, pages, zoom, output_dir, resp) => {
                    match store.export_pages_as_images(&path, &pages, zoom, &output_dir) {
                        Ok(paths) => {
                            let _ = resp.send(Ok(paths));
                        }
                        Err(e) => {
                            let _ = resp.send(Err(e));
                        }
                    }
                }
                PdfCommand::ExportPdf(_, pdf_path, annotations, resp) => {
                    match store.save_annotations(&pdf_path, &annotations) {
                        Ok(saved_path) => {
                            let _ = resp.send(Ok(saved_path));
                        }
                        Err(e) => {
                            let _ = resp.send(Err(e));
                        }
                    }
                }
                PdfCommand::Search(_, query, resp) => match store.search(&path, &query) {
                    Ok(results) => {
                        let _ = resp.send(Ok(results));
                    }
                    Err(e) => {
                        let _ = resp.send(Err(e));
                    }
                },
                PdfCommand::LoadAnnotations(_, pdf_path, resp) => {
                    match store.load_annotations(&pdf_path) {
                        Ok(annotations) => {
                            let _ = resp.send(Ok(annotations));
                        }
                        Err(e) => {
                            let _ = resp.send(Err(e));
                        }
                    }
                }
                PdfCommand::Close(_) => {
                    store.close_document(&path);
                    break; // Terminate thread
                }
            }
        }
        active_workers.fetch_sub(1, Ordering::SeqCst);
    });
}

pub fn spawn_engine_thread(cache_size: u64) -> EngineState {
    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
    let cache = create_render_cache(cache_size);
    let active_workers = Arc::new(AtomicUsize::new(0));

    let pdfium = match Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
        .or_else(|_| Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name()))
    {
        Ok(bindings) => Arc::new(Pdfium::new(bindings)),
        Err(e) => {
            log::error!("Critical Error: Failed to bind to Pdfium library: {}", e);
            panic!("Failed to bind to Pdfium library: {}", e);
        }
    };

    let workers_clone = active_workers.clone();
    let cache_clone = cache.clone();
    let pdfium_clone = pdfium.clone();

    // Dispatcher thread
    thread::spawn(move || {
        let mut document_senders: HashMap<u64, crossbeam_channel::Sender<PdfCommand>> =
            HashMap::new();

        while let Ok(cmd) = cmd_rx.recv() {
            match cmd {
                PdfCommand::Open(path, resp) => {
                    let doc_id = next_doc_id();
                    let (worker_tx, worker_rx) = crossbeam_channel::unbounded();

                    spawn_document_worker(
                        doc_id,
                        path.clone(),
                        worker_rx,
                        cache_clone.clone(),
                        workers_clone.clone(),
                        pdfium_clone.clone(),
                    );

                    document_senders.insert(doc_id.0, worker_tx.clone());
                    // Forward the open command to the new worker
                    let _ = worker_tx.send(PdfCommand::Open(path, resp));
                }
                PdfCommand::Render(doc_id, page, opts, resp) => {
                    if let Some(tx) = document_senders.get(&doc_id.0) {
                        if let Err(e) = tx.send(PdfCommand::Render(doc_id, page, opts, resp)) {
                            log::error!(
                                "Failed to send render command to worker {}: {}",
                                doc_id.0,
                                e
                            );
                            document_senders.remove(&doc_id.0);
                        }
                    }
                }
                PdfCommand::RenderThumbnail(doc_id, page, zoom, resp) => {
                    if let Some(tx) = document_senders.get(&doc_id.0) {
                        if let Err(e) =
                            tx.send(PdfCommand::RenderThumbnail(doc_id, page, zoom, resp))
                        {
                            log::error!(
                                "Failed to send render thumbnail command to worker {}: {}",
                                doc_id.0,
                                e
                            );
                            document_senders.remove(&doc_id.0);
                        }
                    }
                }
                PdfCommand::ExtractText(doc_id, page, resp) => {
                    if let Some(tx) = document_senders.get(&doc_id.0) {
                        if let Err(e) = tx.send(PdfCommand::ExtractText(doc_id, page, resp)) {
                            log::error!(
                                "Failed to send extract text command to worker {}: {}",
                                doc_id.0,
                                e
                            );
                            document_senders.remove(&doc_id.0);
                        }
                    }
                }
                PdfCommand::ExportImage(doc_id, page, zoom, path, resp) => {
                    if let Some(tx) = document_senders.get(&doc_id.0) {
                        let _ = tx.send(PdfCommand::ExportImage(doc_id, page, zoom, path, resp));
                    }
                }
                PdfCommand::ExportImages(doc_id, pages, zoom, dir, resp) => {
                    if let Some(tx) = document_senders.get(&doc_id.0) {
                        let _ = tx.send(PdfCommand::ExportImages(doc_id, pages, zoom, dir, resp));
                    }
                }
                PdfCommand::ExportPdf(doc_id, path, ann, resp) => {
                    if let Some(tx) = document_senders.get(&doc_id.0) {
                        let _ = tx.send(PdfCommand::ExportPdf(doc_id, path, ann, resp));
                    }
                }
                PdfCommand::Search(doc_id, query, resp) => {
                    if let Some(tx) = document_senders.get(&doc_id.0) {
                        let _ = tx.send(PdfCommand::Search(doc_id, query, resp));
                    }
                }
                PdfCommand::LoadAnnotations(doc_id, path, resp) => {
                    if let Some(tx) = document_senders.get(&doc_id.0) {
                        let _ = tx.send(PdfCommand::LoadAnnotations(doc_id, path, resp));
                    }
                }
                PdfCommand::Close(doc_id) => {
                    if let Some(tx) = document_senders.remove(&doc_id.0) {
                        let _ = tx.send(PdfCommand::Close(doc_id));
                    }
                }
            }
        }
    });

    EngineState {
        cmd_tx,
        active_workers,
    }
}
