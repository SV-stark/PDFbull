use crate::commands::PdfCommand;
use crate::models::next_doc_id;
use crate::pdf_engine::{create_render_cache, DocumentStore};
use pdfium_render::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

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

fn spawn_worker(
    cmd_rx: crossbeam_channel::Receiver<PdfCommand>,
    cache: crate::pdf_engine::SharedRenderCache,
    doc_paths: Arc<RwLock<HashMap<u64, String>>>,
    active_workers: Arc<AtomicUsize>,
    pdfium: Arc<Pdfium>,
) {
    active_workers.fetch_add(1, Ordering::SeqCst);

    thread::spawn(move || {
        let mut store = match DocumentStore::new(&pdfium, cache) {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to initialize DocumentStore: {}", e);
                active_workers.fetch_sub(1, Ordering::SeqCst);
                return;
            }
        };

        while let Ok(cmd) = cmd_rx.recv() {
            match cmd {
                PdfCommand::Open(path, resp) => match store.open_document(&path) {
                    Ok((path_str, count, heights, width)) => {
                        let doc_id = next_doc_id();
                        doc_paths.write().unwrap().insert(doc_id.0, path.clone());
                        let outline = store.get_outline(&path_str);
                        let _ = resp.send(Ok((doc_id, count, heights, width, outline)));
                    }
                    Err(e) => {
                        let _ = resp.send(Err(e));
                    }
                },
                PdfCommand::Render(doc_id, page, options, resp) => {
                    let path = doc_paths.read().unwrap().get(&doc_id.0).cloned();
                    if let Some(path) = path {
                        let _ = store.ensure_opened(&path);
                        match store.render_page(&path, page, options) {
                            Ok((w, h, data)) => {
                                let _ = resp.send(Ok((w, h, data)));
                            }
                            Err(e) => {
                                let _ = resp.send(Err(e));
                            }
                        }
                    } else {
                        let _ = resp.send(Err("Document not found".into()));
                    }
                }
                PdfCommand::RenderThumbnail(doc_id, page, zoom, resp) => {
                    let path = doc_paths.read().unwrap().get(&doc_id.0).cloned();
                    if let Some(path) = path {
                        let _ = store.ensure_opened(&path);
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
                    } else {
                        let _ = resp.send(Err("Document not found".into()));
                    }
                }
                PdfCommand::ExtractText(doc_id, page, resp) => {
                    let path = doc_paths.read().unwrap().get(&doc_id.0).cloned();
                    if let Some(path) = path {
                        let _ = store.ensure_opened(&path);
                        match store.extract_text(&path, page) {
                            Ok(text) => {
                                let _ = resp.send(Ok(text));
                            }
                            Err(e) => {
                                let _ = resp.send(Err(e));
                            }
                        }
                    } else {
                        let _ = resp.send(Err("Document not found".into()));
                    }
                }
                PdfCommand::ExportImage(doc_id, page, zoom, output_path, resp) => {
                    let path = doc_paths.read().unwrap().get(&doc_id.0).cloned();
                    if let Some(path) = path {
                        let _ = store.ensure_opened(&path);
                        match store.export_page_as_image(&path, page, zoom, &output_path) {
                            Ok(()) => {
                                let _ = resp.send(Ok(()));
                            }
                            Err(e) => {
                                let _ = resp.send(Err(e));
                            }
                        }
                    } else {
                        let _ = resp.send(Err("Document not found".into()));
                    }
                }
                PdfCommand::ExportImages(doc_id, pages, zoom, output_dir, resp) => {
                    let path = doc_paths.read().unwrap().get(&doc_id.0).cloned();
                    if let Some(path) = path {
                        let _ = store.ensure_opened(&path);
                        match store.export_pages_as_images(&path, &pages, zoom, &output_dir) {
                            Ok(paths) => {
                                let _ = resp.send(Ok(paths));
                            }
                            Err(e) => {
                                let _ = resp.send(Err(e));
                            }
                        }
                    } else {
                        let _ = resp.send(Err("Document not found".into()));
                    }
                }
                PdfCommand::ExportPdf(doc_id, pdf_path, annotations, resp) => {
                    let path = doc_paths.read().unwrap().get(&doc_id.0).cloned();
                    if let Some(path) = path {
                        let _ = store.ensure_opened(&path);
                        match store.save_annotations(&pdf_path, &annotations) {
                            Ok(saved_path) => {
                                let _ = resp.send(Ok(saved_path));
                            }
                            Err(e) => {
                                let _ = resp.send(Err(e));
                            }
                        }
                    } else {
                        let _ = resp.send(Err("Document not found".into()));
                    }
                }
                PdfCommand::Search(doc_id, query, resp) => {
                    let path = doc_paths.read().unwrap().get(&doc_id.0).cloned();
                    if let Some(path) = path {
                        let _ = store.ensure_opened(&path);
                        match store.search(&path, &query) {
                            Ok(results) => {
                                let _ = resp.send(Ok(results));
                            }
                            Err(e) => {
                                let _ = resp.send(Err(e));
                            }
                        }
                    } else {
                        let _ = resp.send(Err("Document not found".into()));
                    }
                }
                PdfCommand::LoadAnnotations(doc_id, pdf_path, resp) => {
                    let path = doc_paths.read().unwrap().get(&doc_id.0).cloned();
                    if let Some(_path) = path {
                        match store.load_annotations(&pdf_path) {
                            Ok(annotations) => {
                                let _ = resp.send(Ok(annotations));
                            }
                            Err(e) => {
                                let _ = resp.send(Err(e));
                            }
                        }
                    } else {
                        let _ = resp.send(Err("Document not found".into()));
                    }
                }
                PdfCommand::Close(doc_id) => {
                    if let Some(path) = doc_paths.write().unwrap().remove(&doc_id.0) {
                        store.close_document(&path);
                    }
                }
            }
        }
        active_workers.fetch_sub(1, Ordering::SeqCst);
    });
}

pub fn spawn_engine_thread(cache_size: u64) -> EngineState {
    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();

    let cache = create_render_cache(cache_size);
    let doc_paths = Arc::new(RwLock::new(HashMap::new()));

    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    let pdfium = match Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
        .or_else(|_| Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name()))
    {
        Ok(bindings) => Arc::new(Pdfium::new(bindings)),
        Err(e) => {
            log::error!("Critical Error: Failed to bind to Pdfium library: {}", e);
            panic!("Failed to bind to Pdfium library: {}", e);
        }
    };

    let active_workers = Arc::new(AtomicUsize::new(0));
    let target_workers = num_threads;

    let cmd_rx = Arc::new(cmd_rx);
    for _ in 0..num_threads {
        let rx = (*cmd_rx).clone();
        let cache = cache.clone();
        let doc_paths = doc_paths.clone();
        let workers = active_workers.clone();
        spawn_worker(rx, cache, doc_paths, workers, pdfium.clone());
    }

    let monitor_active_workers = active_workers.clone();
    let monitor_cmd_rx = (*cmd_rx).clone();
    let monitor_cache = cache.clone();
    let monitor_doc_paths = doc_paths.clone();

    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(2));
        let current = monitor_active_workers.load(Ordering::SeqCst);
        if current < target_workers {
            log::warn!(
                "Worker thread died ({} active, {} target). Spawning replacement.",
                current,
                target_workers
            );
            let rx = monitor_cmd_rx.clone();
            let cache = monitor_cache.clone();
            let doc_paths = monitor_doc_paths.clone();
            let workers = monitor_active_workers.clone();
            spawn_worker(rx, cache, doc_paths, workers, pdfium.clone());
        }
    });

    EngineState {
        cmd_tx,
        active_workers,
    }
}
