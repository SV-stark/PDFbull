use crate::commands::PdfCommand;
use crate::models::{next_doc_id, DocumentId};
use crate::pdf_engine::DocumentStore;
use std::collections::HashMap;
use std::sync::mpsc;

#[derive(Debug, Clone)]
pub struct EngineState {
    pub cmd_tx: mpsc::Sender<PdfCommand>,
}

pub fn spawn_engine_thread(cache_size: u64) -> EngineState {
    let (cmd_tx, cmd_rx) = mpsc::channel();

    std::thread::spawn(move || {
        let mut store = match DocumentStore::new(cache_size) {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to initialize DocumentStore: {}", e);
                return;
            }
        };

        let mut doc_paths: HashMap<DocumentId, String> = HashMap::new();

        while let Ok(cmd) = cmd_rx.recv() {
            match cmd {
                PdfCommand::Open(path, resp) => {
                    match store.open_document(&path) {
                        Ok((path_str, count, heights, width)) => {
                            let doc_id = next_doc_id();
                            doc_paths.insert(doc_id, path.clone());
                            let outline = store.get_outline(&path_str);
                            let _ = resp.send(Ok((doc_id, count, heights, width, outline)));
                        }
                        Err(e) => {
                            let _ = resp.send(Err(e));
                        }
                    }
                }
                PdfCommand::Render(doc_id, page, zoom, rotation, filter, auto_crop, resp) => {
                    if let Some(path) = doc_paths.get(&doc_id) {
                        match store.render_page(path, page, zoom, rotation, filter, auto_crop) {
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
                    if let Some(path) = doc_paths.get(&doc_id) {
                        match store.extract_text(path, page) {
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
                    if let Some(path) = doc_paths.get(&doc_id) {
                        match store.export_page_as_image(path, page, zoom, &output_path) {
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
                    if let Some(path) = doc_paths.get(&doc_id) {
                        match store.export_pages_as_images(path, &pages, zoom, &output_dir) {
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
                    if let Some(path) = doc_paths.get(&doc_id) {
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
                    if let Some(path) = doc_paths.get(&doc_id) {
                        match store.search(path, &query) {
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
                    if let Some(_) = doc_paths.get(&doc_id) {
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
                    if let Some(path) = doc_paths.remove(&doc_id) {
                        store.close_document(&path);
                    }
                }
            }
        }
    });

    EngineState { cmd_tx }
}
