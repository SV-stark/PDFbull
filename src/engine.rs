use crate::commands::PdfCommand;
use crate::models::DocumentId;
use crate::pdf_engine;
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

pub fn spawn_engine_thread() -> EngineState {
    let (cmd_tx, mut cmd_rx) = mpsc::channel(32);

    std::thread::spawn(move || {
        let doc_paths: Arc<Mutex<HashMap<DocumentId, String>>> =
            Arc::new(Mutex::new(HashMap::new()));
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);

        let next_id = || DocumentId(NEXT_ID.fetch_add(1, Ordering::Relaxed));

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
                        let id = next_id();
                        let pdfium = match pdf_engine::PdfEngine::init_pdfium() {
                            Ok(p) => p,
                            Err(e) => {
                                let _ = resp.blocking_send(Err(e));
                                continue;
                            }
                        };
                        let mut engine = pdf_engine::PdfEngine::new(&pdfium);
                        match engine.open_document(&path) {
                            Ok((count, heights, width)) => {
                                let outline = engine.get_outline();
                                doc_paths.lock().await.insert(id, path);

                                let _ = resp.blocking_send(Ok((id, count, heights, width, outline)));
                            }
                            Err(e) => {
                                let _ = resp.blocking_send(Err(e));
                            }
                        }
                    }
                    PdfCommand::Render(doc_id, page, zoom, rotation, filter, auto_crop, resp) => {
                        let path = doc_paths.lock().await.get(&doc_id).cloned();
                        if let Some(path) = path {
                            let pdfium = match pdf_engine::PdfEngine::init_pdfium() {
                                Ok(p) => p,
                                Err(e) => {
                                    let _ = resp.blocking_send(Err(e));
                                    continue;
                                }
                            };
                            let mut engine = pdf_engine::PdfEngine::new(&pdfium);
                            if let Err(e) = engine.open_document(&path) {
                                let _ = resp.blocking_send(Err(e));
                            } else {
                                let res = engine.render_page(page, zoom, rotation, filter, auto_crop);
                                match res {
                                    Ok((w, h, data)) => {
                                        let _ = resp.blocking_send(Ok((page as usize, w, h, data)));
                                    }
                                    Err(e) => {
                                        let _ = resp.blocking_send(Err(e));
                                    }
                                }
                            }
                        } else {
                            let _ = resp.blocking_send(Err("Document not found".into()));
                        }
                    }
                    PdfCommand::ExtractText(doc_id, page, resp) => {
                        let path = doc_paths.lock().await.get(&doc_id).cloned();
                        if let Some(path) = path {
                            let pdfium = match pdf_engine::PdfEngine::init_pdfium() {
                                Ok(p) => p,
                                Err(e) => {
                                    let _ = resp.blocking_send(Err(e));
                                    continue;
                                }
                            };
                            let mut engine = pdf_engine::PdfEngine::new(&pdfium);
                            let res = if engine.open_document(&path).is_ok() {
                                engine.extract_text(page)
                            } else {
                                Err("Failed to open document".into())
                            };
                            let _ = resp.blocking_send(res);
                        } else {
                            let _ = resp.blocking_send(Err("Document not found".into()));
                        }
                    }
                    PdfCommand::ExportImage(doc_id, page, zoom, path, resp) => {
                        let doc_path = doc_paths.lock().await.get(&doc_id).cloned();
                        if let Some(doc_path) = doc_path {
                            let pdfium = match pdf_engine::PdfEngine::init_pdfium() {
                                Ok(p) => p,
                                Err(e) => {
                                    let _ = resp.blocking_send(Err(e));
                                    continue;
                                }
                            };
                            let mut engine = pdf_engine::PdfEngine::new(&pdfium);
                            let res = if engine.open_document(&doc_path).is_ok() {
                                engine.export_page_as_image(page, zoom, &path)
                            } else {
                                Err("Failed to open document".into())
                            };
                            let _ = resp.blocking_send(res);
                        } else {
                            let _ = resp.blocking_send(Err("Document not found".into()));
                        }
                    }
                    PdfCommand::ExportImages(doc_id, pages, zoom, output_dir, resp) => {
                        let path = doc_paths.lock().await.get(&doc_id).cloned();
                        if let Some(path) = path {
                            let pdfium = match pdf_engine::PdfEngine::init_pdfium() {
                                Ok(p) => p,
                                Err(e) => {
                                    let _ = resp.blocking_send(Err(e));
                                    continue;
                                }
                            };
                            let mut engine = pdf_engine::PdfEngine::new(&pdfium);
                            let res = if engine.open_document(&path).is_ok() {
                                engine.export_pages_as_images(&pages, zoom, &output_dir)
                            } else {
                                Err("Failed to open document".into())
                            };
                            let _ = resp.blocking_send(res);
                        } else {
                            let _ = resp.blocking_send(Err("Document not found".into()));
                        }
                    }
                    PdfCommand::ExportPdf(doc_id, pdf_path, annotations, resp) => {
                        let path = doc_paths.lock().await.get(&doc_id).cloned();
                        if let Some(path) = path {
                            let pdfium = match pdf_engine::PdfEngine::init_pdfium() {
                                Ok(p) => p,
                                Err(e) => {
                                    let _ = resp.blocking_send(Err(e));
                                    continue;
                                }
                            };
                            let mut engine = pdf_engine::PdfEngine::new(&pdfium);
                            let res = if engine.open_document(&path).is_ok() {
                                engine.save_annotations(&annotations, &pdf_path)
                            } else {
                                Err("Failed to open document".into())
                            };
                            let _ = resp.blocking_send(res);
                        } else {
                            let _ = resp.blocking_send(Err("Document not found".into()));
                        }
                    }
                    PdfCommand::Search(doc_id, query, resp) => {
                        let path = doc_paths.lock().await.get(&doc_id).cloned();
                        if let Some(path) = path {
                            let pdfium = match pdf_engine::PdfEngine::init_pdfium() {
                                Ok(p) => p,
                                Err(e) => {
                                    let _ = resp.blocking_send(Err(e));
                                    continue;
                                }
                            };
                            let mut engine = pdf_engine::PdfEngine::new(&pdfium);
                            let res = if engine.open_document(&path).is_ok() {
                                engine.search(&query, None)
                            } else {
                                Err("Failed to open document".into())
                            };
                            let _ = resp.blocking_send(res);
                        } else {
                            let _ = resp.blocking_send(Err("Document not found".into()));
                        }
                    }
                    PdfCommand::LoadAnnotations(_doc_id, pdf_path, resp) => {
                        let path = doc_paths.lock().await.values().next().cloned();
                        if let Some(path) = path {
                            let pdfium = match pdf_engine::PdfEngine::init_pdfium() {
                                Ok(p) => p,
                                Err(e) => {
                                    let _ = resp.blocking_send(Err(e));
                                    continue;
                                }
                            };
                            let mut engine = pdf_engine::PdfEngine::new(&pdfium);
                            let res = if engine.open_document(&path).is_ok() {
                                engine.load_annotations(&pdf_path)
                            } else {
                                Err("Failed to open document".into())
                            };
                            let _ = resp.blocking_send(res);
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
