use crate::commands::PdfCommand;
use crate::models::DocumentId;
use crate::pdf_engine::{DocumentStore, RenderFilter};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

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
    let (cmd_tx, mut cmd_rx) = mpsc::channel(32);

    std::thread::spawn(move || {
        let doc_paths: Arc<Mutex<HashMap<DocumentId, String>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let rt = match tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("Failed to create Tokio runtime: {}", e);
                return;
            }
        };

        rt.block_on(async move {
            while let Some(cmd) = cmd_rx.recv().await {
                let doc_paths = doc_paths.clone();
                match cmd {
                    PdfCommand::Open(path, resp) => {
                        let store = match DocumentStore::new() {
                            Ok(s) => s,
                            Err(e) => {
                                let _ = resp.blocking_send(Err(e));
                                continue;
                            }
                        };
                        
                        let store = Arc::new(Mutex::new(store));
                        let store_clone = store.clone();
                        let path_clone = path.clone();
                        
                        let result = tokio::task::spawn_blocking(move || {
                            let mut store = store_clone.blocking_lock();
                            store.open_document(&path_clone)
                        }).await;

                        match result {
                            Ok(Ok((path_str, count, heights, width))) => {
                                let doc_id = next_doc_id();
                                doc_paths.lock().await.insert(doc_id, path.clone());
                                
                                let outline = store.lock().await.get_outline(&path_str);
                                
                                let _ = resp.blocking_send(Ok((doc_id, count, heights, width, outline)));
                            }
                            Ok(Err(e)) => {
                                let _ = resp.blocking_send(Err(e));
                            }
                            Err(e) => {
                                let _ = resp.blocking_send(Err(format!("Task join error: {}", e)));
                            }
                        }
                    }
                    PdfCommand::Render(doc_id, page, zoom, rotation, filter, auto_crop, resp) => {
                        let path = doc_paths.lock().await.get(&doc_id).cloned();
                        if let Some(path) = path {
                            let store = match DocumentStore::new() {
                                Ok(s) => s,
                                Err(e) => {
                                    let _ = resp.blocking_send(Err(e));
                                    continue;
                                }
                            };
                            
                            let store = Arc::new(Mutex::new(store));
                            let store_clone = store.clone();
                            
                            let result = tokio::task::spawn_blocking(move || {
                                let mut store = store_clone.blocking_lock();
                                store.open_document(&path)?;
                                store.render_page(&path, page, zoom, rotation, filter, auto_crop)
                            }).await;

                            match result {
                                Ok(Ok((w, h, data))) => {
                                    let _ = resp.blocking_send(Ok((page as usize, w, h, data)));
                                }
                                Ok(Err(e)) => {
                                    let _ = resp.blocking_send(Err(e));
                                }
                                Err(e) => {
                                    let _ = resp.blocking_send(Err(format!("Task join error: {}", e)));
                                }
                            }
                        } else {
                            let _ = resp.blocking_send(Err("Document not found".into()));
                        }
                    }
                    PdfCommand::ExtractText(doc_id, page, resp) => {
                        let path = doc_paths.lock().await.get(&doc_id).cloned();
                        if let Some(path) = path {
                            let store = match DocumentStore::new() {
                                Ok(s) => s,
                                Err(e) => {
                                    let _ = resp.blocking_send(Err(e));
                                    continue;
                                }
                            };
                            
                            let store = Arc::new(Mutex::new(store));
                            let store_clone = store.clone();
                            
                            let result = tokio::task::spawn_blocking(move || {
                                let mut store = store_clone.blocking_lock();
                                store.open_document(&path)?;
                                store.extract_text(&path, page)
                            }).await;

                            match result {
                                Ok(Ok(text)) => {
                                    let _ = resp.blocking_send(Ok(text));
                                }
                                Ok(Err(e)) => {
                                    let _ = resp.blocking_send(Err(e));
                                }
                                Err(e) => {
                                    let _ = resp.blocking_send(Err(format!("Task join error: {}", e)));
                                }
                            }
                        } else {
                            let _ = resp.blocking_send(Err("Document not found".into()));
                        }
                    }
                    PdfCommand::ExportImage(doc_id, page, zoom, path, resp) => {
                        let doc_path = doc_paths.lock().await.get(&doc_id).cloned();
                        if let Some(doc_path) = doc_path {
                            let store = match DocumentStore::new() {
                                Ok(s) => s,
                                Err(e) => {
                                    let _ = resp.blocking_send(Err(e));
                                    continue;
                                }
                            };
                            
                            let store = Arc::new(Mutex::new(store));
                            let store_clone = store.clone();
                            
                            let result = tokio::task::spawn_blocking(move || {
                                let mut store = store_clone.blocking_lock();
                                store.open_document(&doc_path)?;
                                store.export_page_as_image(&doc_path, page, zoom, &path)
                            }).await;

                            match result {
                                Ok(Ok(())) => {
                                    let _ = resp.blocking_send(Ok(()));
                                }
                                Ok(Err(e)) => {
                                    let _ = resp.blocking_send(Err(e));
                                }
                                Err(e) => {
                                    let _ = resp.blocking_send(Err(format!("Task join error: {}", e)));
                                }
                            }
                        } else {
                            let _ = resp.blocking_send(Err("Document not found".into()));
                        }
                    }
                    PdfCommand::ExportImages(doc_id, pages, zoom, output_dir, resp) => {
                        let path = doc_paths.lock().await.get(&doc_id).cloned();
                        if let Some(path) = path {
                            let store = match DocumentStore::new() {
                                Ok(s) => s,
                                Err(e) => {
                                    let _ = resp.blocking_send(Err(e));
                                    continue;
                                }
                            };
                            
                            let store = Arc::new(Mutex::new(store));
                            let store_clone = store.clone();
                            
                            let result = tokio::task::spawn_blocking(move || {
                                let mut store = store_clone.blocking_lock();
                                store.open_document(&path)?;
                                store.export_pages_as_images(&path, &pages, zoom, &output_dir)
                            }).await;

                            match result {
                                Ok(Ok(paths)) => {
                                    let _ = resp.blocking_send(Ok(paths));
                                }
                                Ok(Err(e)) => {
                                    let _ = resp.blocking_send(Err(e));
                                }
                                Err(e) => {
                                    let _ = resp.blocking_send(Err(format!("Task join error: {}", e)));
                                }
                            }
                        } else {
                            let _ = resp.blocking_send(Err("Document not found".into()));
                        }
                    }
                    PdfCommand::ExportPdf(doc_id, pdf_path, annotations, resp) => {
                        let path = doc_paths.lock().await.get(&doc_id).cloned();
                        if let Some(_path) = path {
                            let store = match DocumentStore::new() {
                                Ok(s) => s,
                                Err(e) => {
                                    let _ = resp.blocking_send(Err(e));
                                    continue;
                                }
                            };
                            
                            let store = Arc::new(Mutex::new(store));
                            let store_clone = store.clone();
                            
                            let result = tokio::task::spawn_blocking(move || {
                                let mut store = store_clone.blocking_lock();
                                store.save_annotations(&pdf_path, &annotations)
                            }).await;

                            match result {
                                Ok(Ok(path)) => {
                                    let _ = resp.blocking_send(Ok(path));
                                }
                                Ok(Err(e)) => {
                                    let _ = resp.blocking_send(Err(e));
                                }
                                Err(e) => {
                                    let _ = resp.blocking_send(Err(format!("Task join error: {}", e)));
                                }
                            }
                        } else {
                            let _ = resp.blocking_send(Err("Document not found".into()));
                        }
                    }
                    PdfCommand::Search(doc_id, query, resp) => {
                        let path = doc_paths.lock().await.get(&doc_id).cloned();
                        if let Some(path) = path {
                            let store = match DocumentStore::new() {
                                Ok(s) => s,
                                Err(e) => {
                                    let _ = resp.blocking_send(Err(e));
                                    continue;
                                }
                            };
                            
                            let store = Arc::new(Mutex::new(store));
                            let store_clone = store.clone();
                            
                            let result = tokio::task::spawn_blocking(move || {
                                let mut store = store_clone.blocking_lock();
                                store.open_document(&path)?;
                                store.search(&path, &query)
                            }).await;

                            match result {
                                Ok(Ok(results)) => {
                                    let _ = resp.blocking_send(Ok(results));
                                }
                                Ok(Err(e)) => {
                                    let _ = resp.blocking_send(Err(e));
                                }
                                Err(e) => {
                                    let _ = resp.blocking_send(Err(format!("Task join error: {}", e)));
                                }
                            }
                        } else {
                            let _ = resp.blocking_send(Err("Document not found".into()));
                        }
                    }
                    PdfCommand::LoadAnnotations(_doc_id, pdf_path, resp) => {
                        let path = doc_paths.lock().await.values().next().cloned();
                        if let Some(_path) = path {
                            let store = match DocumentStore::new() {
                                Ok(s) => s,
                                Err(e) => {
                                    let _ = resp.blocking_send(Err(e));
                                    continue;
                                }
                            };
                            
                            let store = Arc::new(Mutex::new(store));
                            let store_clone = store.clone();
                            
                            let result = tokio::task::spawn_blocking(move || {
                                let store = store_clone.blocking_lock();
                                store.load_annotations(&pdf_path)
                            }).await;

                            match result {
                                Ok(Ok(annotations)) => {
                                    let _ = resp.blocking_send(Ok(annotations));
                                }
                                Ok(Err(e)) => {
                                    let _ = resp.blocking_send(Err(e));
                                }
                                Err(e) => {
                                    let _ = resp.blocking_send(Err(format!("Task join error: {}", e)));
                                }
                            }
                        } else {
                            let _ = resp.blocking_send(Err("No document found".into()));
                        }
                    }
                    PdfCommand::Close(doc_id) => {
                        doc_paths.lock().await.remove(&doc_id);
                    }
                }
            }
        });
    });

    EngineState {
        cmd_tx,
        documents: HashMap::new(),
    }
}
