use crate::commands::PdfCommand;
use crate::models::{next_doc_id, DocumentId};
use crate::pdf_engine::{create_render_cache, DocumentStore, RenderFilter, RenderQuality};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct EngineState {
    pub cmd_tx: crossbeam_channel::Sender<PdfCommand>,
}

pub fn spawn_engine_thread(cache_size: u64) -> EngineState {
    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();

    let cache = create_render_cache(cache_size);
    let doc_paths = Arc::new(RwLock::new(HashMap::new()));

    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    for _ in 0..num_threads {
        let rx = cmd_rx.clone();
        let cache = cache.clone();
        let doc_paths = doc_paths.clone();

        std::thread::spawn(move || {
            let bindings = pdfium_render::prelude::Pdfium::bind_to_library(
                pdfium_render::prelude::Pdfium::pdfium_platform_library_name_at_path("./"),
            )
            .or_else(|_| {
                pdfium_render::prelude::Pdfium::bind_to_library(
                    pdfium_render::prelude::Pdfium::pdfium_platform_library_name(),
                )
            })
            .expect("Failed to bind to Pdfium library");
            let pdfium = pdfium_render::prelude::Pdfium::new(bindings);

            let mut store = match DocumentStore::new(&pdfium, cache) {
                Ok(s) => s,
                Err(e) => {
                    log::error!("Failed to initialize DocumentStore: {}", e);
                    return;
                }
            };

            while let Ok(cmd) = rx.recv() {
                match cmd {
                    PdfCommand::Open(path, resp) => match store.open_document(&path) {
                        Ok((path_str, count, heights, width)) => {
                            let doc_id = next_doc_id();
                            doc_paths.write().unwrap().insert(doc_id, path.clone());
                            let outline = store.get_outline(&path_str);
                            let _ = resp.send(Ok((doc_id, count, heights, width, outline)));
                        }
                        Err(e) => {
                            let _ = resp.send(Err(e));
                        }
                    },
                    PdfCommand::Render(
                        doc_id,
                        page,
                        zoom,
                        rotation,
                        filter,
                        auto_crop,
                        quality,
                        resp,
                    ) => {
                        let path = doc_paths.read().unwrap().get(&doc_id).cloned();
                        if let Some(path) = path {
                            let _ = store.ensure_opened(&path);
                            match store.render_page(
                                &path, page, zoom, rotation, filter, auto_crop, quality,
                            ) {
                                Ok((w, h, data)) => {
                                    let _ = resp.send(Ok((page as usize, w, h, data)));
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
                        let path = doc_paths.read().unwrap().get(&doc_id).cloned();
                        if let Some(path) = path {
                            let _ = store.ensure_opened(&path);
                            match store.render_page(
                                &path,
                                page,
                                zoom,
                                0,
                                RenderFilter::None,
                                false,
                                RenderQuality::Low,
                            ) {
                                Ok((w, h, data)) => {
                                    let _ = resp.send(Ok((page as usize, w, h, data)));
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
                        let path = doc_paths.read().unwrap().get(&doc_id).cloned();
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
                        let path = doc_paths.read().unwrap().get(&doc_id).cloned();
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
                        let path = doc_paths.read().unwrap().get(&doc_id).cloned();
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
                        let path = doc_paths.read().unwrap().get(&doc_id).cloned();
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
                        let path = doc_paths.read().unwrap().get(&doc_id).cloned();
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
                        let path = doc_paths.read().unwrap().get(&doc_id).cloned();
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
                        if let Some(path) = doc_paths.write().unwrap().remove(&doc_id) {
                            store.close_document(&path);
                        }
                    }
                }
            }
        });
    }

    EngineState { cmd_tx }
}
