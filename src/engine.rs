use crate::commands::PdfCommand;
use crate::models::DocumentId;
use crate::pdf_engine::DocumentStore;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};

#[derive(Debug, Clone)]
pub struct EngineState {
    pub cmd_tx: mpsc::Sender<PdfCommand>,
    pub documents: HashMap<DocumentId, String>,
}

static NEXT_DOC_ID: AtomicU64 = AtomicU64::new(1);

fn next_doc_id() -> DocumentId {
    DocumentId(NEXT_DOC_ID.fetch_add(1, Ordering::Relaxed))
}

pub fn spawn_engine_thread() -> EngineState {
    let (cmd_tx, cmd_rx) = mpsc::channel();

    std::thread::spawn(move || {
        let doc_paths: Arc<Mutex<HashMap<DocumentId, String>>> =
            Arc::new(Mutex::new(HashMap::new()));

        while let Ok(cmd) = cmd_rx.recv() {
            let doc_paths = doc_paths.clone();
            match cmd {
                PdfCommand::Open(path, resp) => {
                    let path_clone = path.clone();
                    rayon::spawn(move || {
                        let mut store = match DocumentStore::new() {
                            Ok(s) => s,
                            Err(e) => {
                                let _ = resp.send(Err(e));
                                return;
                            }
                        };

                        match store.open_document(&path_clone) {
                            Ok((path_str, count, heights, width)) => {
                                let doc_id = next_doc_id();
                                if let Ok(mut lock) = doc_paths.lock() {
                                    lock.insert(doc_id, path.clone());
                                }

                                let outline = match DocumentStore::new() {
                                    Ok(store) => store.get_outline(&path_str).unwrap_or_default(),
                                    Err(_) => Vec::new(),
                                };

                                let _ = resp.send(Ok((doc_id, count, heights, width, outline)));
                            }
                            Err(e) => {
                                let _ = resp.send(Err(e));
                            }
                        }
                    });
                }
                PdfCommand::Render(doc_id, page, zoom, rotation, filter, auto_crop, resp) => {
                    let path = doc_paths
                        .lock()
                        .ok()
                        .and_then(|lock| lock.get(&doc_id).cloned());
                    if let Some(path) = path {
                        rayon::spawn(move || {
                            let mut store = match DocumentStore::new() {
                                Ok(s) => s,
                                Err(e) => {
                                    let _ = resp.send(Err(e));
                                    return;
                                }
                            };

                            if let Err(e) = store.open_document(&path) {
                                let _ = resp.send(Err(e));
                                return;
                            }

                            match store.render_page(&path, page, zoom, rotation, filter, auto_crop)
                            {
                                Ok((w, h, data)) => {
                                    let _ = resp.send(Ok((page as usize, w, h, data)));
                                }
                                Err(e) => {
                                    let _ = resp.send(Err(e));
                                }
                            }
                        });
                    } else {
                        let _ = resp.send(Err("Document not found".into()));
                    }
                }
                PdfCommand::ExtractText(doc_id, page, resp) => {
                    let path = doc_paths
                        .lock()
                        .ok()
                        .and_then(|lock| lock.get(&doc_id).cloned());
                    if let Some(path) = path {
                        rayon::spawn(move || {
                            let mut store = match DocumentStore::new() {
                                Ok(s) => s,
                                Err(e) => {
                                    let _ = resp.send(Err(e));
                                    return;
                                }
                            };
                            if let Err(e) = store.open_document(&path) {
                                let _ = resp.send(Err(e));
                                return;
                            }
                            match store.extract_text(&path, page) {
                                Ok(text) => {
                                    let _ = resp.send(Ok(text));
                                }
                                Err(e) => {
                                    let _ = resp.send(Err(e));
                                }
                            }
                        });
                    } else {
                        let _ = resp.send(Err("Document not found".into()));
                    }
                }
                PdfCommand::ExportImage(doc_id, page, zoom, output_path, resp) => {
                    let doc_path = doc_paths
                        .lock()
                        .ok()
                        .and_then(|lock| lock.get(&doc_id).cloned());
                    if let Some(doc_path) = doc_path {
                        rayon::spawn(move || {
                            let mut store = match DocumentStore::new() {
                                Ok(s) => s,
                                Err(e) => {
                                    let _ = resp.send(Err(e));
                                    return;
                                }
                            };
                            if let Err(e) = store.open_document(&doc_path) {
                                let _ = resp.send(Err(e));
                                return;
                            }
                            match store.export_page_as_image(&doc_path, page, zoom, &output_path) {
                                Ok(()) => {
                                    let _ = resp.send(Ok(()));
                                }
                                Err(e) => {
                                    let _ = resp.send(Err(e));
                                }
                            }
                        });
                    } else {
                        let _ = resp.send(Err("Document not found".into()));
                    }
                }
                PdfCommand::ExportImages(doc_id, pages, zoom, output_dir, resp) => {
                    let path = doc_paths
                        .lock()
                        .ok()
                        .and_then(|lock| lock.get(&doc_id).cloned());
                    if let Some(path) = path {
                        rayon::spawn(move || {
                            let mut store = match DocumentStore::new() {
                                Ok(s) => s,
                                Err(e) => {
                                    let _ = resp.send(Err(e));
                                    return;
                                }
                            };
                            if let Err(e) = store.open_document(&path) {
                                let _ = resp.send(Err(e));
                                return;
                            }
                            match store.export_pages_as_images(&path, &pages, zoom, &output_dir) {
                                Ok(paths) => {
                                    let _ = resp.send(Ok(paths));
                                }
                                Err(e) => {
                                    let _ = resp.send(Err(e));
                                }
                            }
                        });
                    } else {
                        let _ = resp.send(Err("Document not found".into()));
                    }
                }
                PdfCommand::ExportPdf(doc_id, pdf_path, annotations, resp) => {
                    let path = doc_paths
                        .lock()
                        .ok()
                        .and_then(|lock| lock.get(&doc_id).cloned());
                    if let Some(_path) = path {
                        rayon::spawn(move || {
                            let store = match DocumentStore::new() {
                                Ok(s) => s,
                                Err(e) => {
                                    let _ = resp.send(Err(e));
                                    return;
                                }
                            };
                            match store.save_annotations(&pdf_path, &annotations) {
                                Ok(path) => {
                                    let _ = resp.send(Ok(path));
                                }
                                Err(e) => {
                                    let _ = resp.send(Err(e));
                                }
                            }
                        });
                    } else {
                        let _ = resp.send(Err("Document not found".into()));
                    }
                }
                PdfCommand::Search(doc_id, query, resp) => {
                    let path = doc_paths
                        .lock()
                        .ok()
                        .and_then(|lock| lock.get(&doc_id).cloned());
                    if let Some(path) = path {
                        rayon::spawn(move || {
                            let mut store = match DocumentStore::new() {
                                Ok(s) => s,
                                Err(e) => {
                                    let _ = resp.send(Err(e));
                                    return;
                                }
                            };
                            if let Err(e) = store.open_document(&path) {
                                let _ = resp.send(Err(e));
                                return;
                            }
                            match store.search(&path, &query) {
                                Ok(results) => {
                                    let _ = resp.send(Ok(results));
                                }
                                Err(e) => {
                                    let _ = resp.send(Err(e));
                                }
                            }
                        });
                    } else {
                        let _ = resp.send(Err("Document not found".into()));
                    }
                }
                PdfCommand::LoadAnnotations(_doc_id, pdf_path, resp) => {
                    let path = doc_paths
                        .lock()
                        .ok()
                        .and_then(|lock| lock.values().next().cloned());
                    if let Some(_path) = path {
                        rayon::spawn(move || {
                            let store = match DocumentStore::new() {
                                Ok(s) => s,
                                Err(e) => {
                                    let _ = resp.send(Err(e));
                                    return;
                                }
                            };
                            match store.load_annotations(&pdf_path) {
                                Ok(annotations) => {
                                    let _ = resp.send(Ok(annotations));
                                }
                                Err(e) => {
                                    let _ = resp.send(Err(e));
                                }
                            }
                        });
                    } else {
                        let _ = resp.send(Err("No document found".into()));
                    }
                }
                PdfCommand::Close(doc_id) => {
                    if let Ok(mut lock) = doc_paths.lock() {
                        lock.remove(&doc_id);
                    }
                }
            }
        }
    });

    EngineState {
        cmd_tx,
        documents: HashMap::new(),
    }
}
