use crate::app::PdfBullApp;
use crate::commands::PdfCommand;
use crate::message::Message;
use iced::Task;

pub fn handle_export_message(app: &mut PdfBullApp, message: Message) -> Task<Message> {
    match message {
        Message::ExtractText => {
            let Some(tab) = app.current_tab() else {
                return Task::none();
            };

            let page = tab.current_page as i32;
            let doc_id = tab.id;

            let Some(engine) = &app.engine else {
                return Task::none();
            };

            let cmd_tx = engine.cmd_tx.clone();
            Task::perform(
                async move {
                    let file = rfd::AsyncFileDialog::new()
                        .add_filter("Text", &["txt"])
                        .set_file_name("extracted_text.txt")
                        .save_file()
                        .await;

                    match file {
                        Some(f) => {
                            let path = f.path().to_path_buf();
                            let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                            if let Err(e) =
                                cmd_tx.send(PdfCommand::ExtractText(doc_id, page, resp_tx))
                            {
                                tracing::error!("Failed to send ExtractText command: {e}");
                                return Err(crate::models::PdfError::EngineDied);
                            }
                            match resp_rx.await {
                                Ok(Ok(text)) => {
                                    if let Err(e) = std::fs::write(&path, &text) {
                                        Err(crate::models::PdfError::from(format!(
                                            "Failed to write file: {e}"
                                        )))
                                    } else {
                                        Ok(path.to_string_lossy().to_string())
                                    }
                                }
                                Ok(Err(e)) => Err(e),
                                Err(_) => Err(crate::models::PdfError::EngineDied),
                            }
                        }
                        None => Err(crate::models::PdfError::from("Cancelled")),
                    }
                },
                Message::TextExtracted,
            )
        }
        Message::ExtractTextToClipboard => {
            let Some(tab) = app.current_tab() else {
                return Task::none();
            };

            let page = tab.current_page as i32;
            let doc_id = tab.id;

            let Some(engine) = &app.engine else {
                return Task::none();
            };

            let cmd_tx = engine.cmd_tx.clone();
            Task::perform(
                async move {
                    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                    if let Err(e) = cmd_tx.send(PdfCommand::ExtractText(doc_id, page, resp_tx)) {
                        tracing::error!("Failed to send ExtractText command: {e}");
                        return Err(crate::models::PdfError::EngineDied);
                    }
                    match resp_rx.await {
                        Ok(Ok(text)) => Ok(text),
                        Ok(Err(e)) => Err(e),
                        Err(_) => Err(crate::models::PdfError::EngineDied),
                    }
                },
                |res| match res {
                    Ok(text) => Message::CopyToClipboard(text),
                    Err(e) => Message::Error(format!("Extraction failed: {e}")),
                },
            )
        }
        Message::CopyToClipboard(text) => {
            let Ok(mut clipboard) = arboard::Clipboard::new() else {
                return app.update(Message::Error("Clipboard error".into()));
            };
            if let Err(e) = clipboard.set_text(text) {
                return app.update(Message::Error(format!("Failed to copy: {e}")));
            }
            app.status_message = Some("Copied to clipboard".into());
            Task::none()
        }
        Message::CopyImageToClipboard => {
            let Some(tab) = app.current_tab() else {
                return Task::none();
            };

            let page = tab.current_page;
            let doc_id = tab.id;
            let zoom = tab.zoom;
            let rotation = tab.rotation;
            let filter = tab.render_filter;
            let auto_crop = tab.auto_crop;
            let quality = app.settings.render_quality;

            let Some(engine) = &app.engine else {
                return Task::none();
            };

            let cmd_tx = engine.cmd_tx.clone();
            Task::perform(
                async move {
                    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                    let options = crate::pdf_engine::RenderOptions {
                        scale: zoom,
                        rotation,
                        filter,
                        auto_crop,
                        quality,
                    };
                    if let Err(_e) = cmd_tx.send(crate::commands::PdfCommand::Render(
                        doc_id, page, options, resp_tx,
                    )) {
                        return Err(crate::models::PdfError::EngineDied);
                    }
                    match resp_rx.await {
                        Ok(Ok(res)) => {
                            let mut clipboard = arboard::Clipboard::new()
                                .map_err(|e| crate::models::PdfError::from(e.to_string()))?;
                            let image_data = arboard::ImageData {
                                width: res.width as usize,
                                height: res.height as usize,
                                bytes: std::borrow::Cow::Borrowed(&res.data),
                            };
                            clipboard
                                .set_image(image_data)
                                .map_err(|e| crate::models::PdfError::from(e.to_string()))?;
                            Ok(())
                        }
                        Ok(Err(e)) => Err(e),
                        Err(_) => Err(crate::models::PdfError::EngineDied),
                    }
                },
                |res| match res {
                    Ok(_) => Message::ClearStatus,
                    Err(e) => Message::Error(format!("Copy image failed: {e}")),
                },
            )
        }
        Message::TextExtracted(result) => {
            match result {
                Ok(path) => app.status_message = Some(format!("Text extracted to: {path}")),
                Err(e) => {
                    if e != "Cancelled" {
                        tracing::error!("Text extraction error: {e}");
                        if e == "Engine died" || e == "Channel closed" {
                            app.engine = None;
                            app.status_message = Some(
                                "PDF engine crashed. Please try your action again to restart it."
                                    .into(),
                            );
                        } else {
                            app.status_message = Some(format!("Text extraction error: {e}"));
                        }
                    }
                }
            }
            Task::none()
        }
        Message::ExportImage => {
            let Some(tab) = app.current_tab() else {
                return Task::none();
            };

            let page = tab.current_page as i32;
            let zoom = tab.zoom;
            let doc_id = tab.id;

            let Some(engine) = &app.engine else {
                return Task::none();
            };

            let cmd_tx = engine.cmd_tx.clone();
            Task::perform(
                async move {
                    let file = rfd::AsyncFileDialog::new()
                        .add_filter("PNG", &["png"])
                        .set_file_name("page.png")
                        .save_file()
                        .await;

                    match file {
                        Some(f) => {
                            let path = f.path().to_path_buf();
                            let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                            if let Err(e) =
                                cmd_tx.send(PdfCommand::ExportImage(doc_id, page, zoom, resp_tx))
                            {
                                tracing::error!("Failed to send ExportImage command: {e}");
                                return Err(crate::models::PdfError::EngineDied);
                            }
                            match resp_rx.await {
                                Ok(Ok(buf)) => tokio::task::spawn_blocking(move || {
                                    let optimized = oxipng::optimize_from_memory(
                                        &buf,
                                        &oxipng::Options::default(),
                                    )
                                    .unwrap_or(buf);
                                    std::fs::write(&path, optimized).map_err(|e| {
                                        crate::models::PdfError::from(format!(
                                            "Failed to write file: {e}"
                                        ))
                                    })?;
                                    Ok(path.to_string_lossy().to_string())
                                })
                                .await
                                .map_err(|e| crate::models::PdfError::from(e.to_string()))?,
                                Ok(Err(e)) => Err(e),
                                Err(_) => Err(crate::models::PdfError::EngineDied),
                            }
                        }
                        None => Err(crate::models::PdfError::from("Cancelled")),
                    }
                },
                Message::ImageExported,
            )
        }
        Message::ImageExported(result) => {
            match result {
                Ok(path) => app.status_message = Some(format!("Exported to: {path}")),
                Err(e) => {
                    tracing::error!("Export error: {e}");
                    if e == "Engine died" || e == "Channel closed" {
                        app.engine = None;
                        app.status_message =
                            Some("PDF engine crashed. Please try your action again.".into());
                    } else if e != "Cancelled" {
                        app.status_message = Some(format!("Export error: {e}"));
                    }
                }
            }
            Task::none()
        }
        Message::ExportImages => {
            let Some(tab) = app.current_tab() else {
                return Task::none();
            };

            let total_pages = tab.total_pages;
            let zoom = tab.zoom;
            let doc_id = tab.id;

            let Some(engine) = &app.engine else {
                return Task::none();
            };

            let cmd_tx = engine.cmd_tx.clone();
            Task::perform(
                async move {
                    let folder = rfd::AsyncFileDialog::new().pick_folder().await;

                    match folder {
                        Some(f) => {
                            let path = f.path().to_string_lossy().to_string();
                            let pages: Vec<i32> = (0..total_pages as i32).collect();
                            let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                            if let Err(e) = cmd_tx.send(PdfCommand::ExportImages(
                                doc_id,
                                pages,
                                zoom,
                                path.clone(),
                                resp_tx,
                            )) {
                                tracing::error!("Failed to send ExportImages command: {e}");
                                return Err(crate::models::PdfError::EngineError(
                                    "Engine died".into(),
                                ));
                            }
                            match resp_rx.await {
                                Ok(Ok(paths)) => Ok(paths.join(", ")),
                                Ok(Err(e)) => Err(e),
                                Err(_) => {
                                    Err(crate::models::PdfError::EngineError("Engine died".into()))
                                }
                            }
                        }
                        None => Err(crate::models::PdfError::OpenFailed("Cancelled".into())),
                    }
                },
                |res| match res {
                    Ok(p) => Message::ImageExported(Ok(p)),
                    Err(e) => Message::ImageExported(Err(e)),
                },
            )
        }
        Message::MergeDocuments(paths) => {
            let engine = if let Some(e) = &app.engine {
                e
            } else {
                let cache_size = app.settings.cache_size as u64;
                let max_mem = app.settings.max_cache_memory as u64;
                app.engine = Some(crate::engine::spawn_engine_thread(cache_size, max_mem));
                app.engine.as_ref().unwrap()
            };
            let cmd_tx = engine.cmd_tx.clone();
            Task::perform(
                async move {
                    let mut final_paths = paths;
                    if final_paths.is_empty() {
                        let picked = rfd::AsyncFileDialog::new()
                            .add_filter("PDF", &["pdf"])
                            .set_title("Select PDFs to Merge")
                            .pick_files()
                            .await;

                        if let Some(files) = picked {
                            final_paths =
                                files.into_iter().map(|f| f.path().to_path_buf()).collect();
                        } else {
                            return Err(crate::models::PdfError::OpenFailed("Cancelled".into()));
                        }
                    }

                    if final_paths.len() < 2 {
                        return Err(crate::models::PdfError::OpenFailed(
                            "Please select at least 2 files to merge".into(),
                        ));
                    }

                    let out = rfd::AsyncFileDialog::new()
                        .add_filter("PDF", &["pdf"])
                        .set_file_name("merged.pdf")
                        .set_title("Save Merged PDF")
                        .save_file()
                        .await;

                    if let Some(f) = out {
                        let path_strs: Vec<String> = final_paths
                            .iter()
                            .map(|p| p.to_string_lossy().to_string())
                            .collect();
                        let (tx, rx) = tokio::sync::oneshot::channel();

                        cmd_tx
                            .send(PdfCommand::Merge(
                                path_strs,
                                f.path().to_string_lossy().to_string(),
                                tx,
                            ))
                            .map_err(|_| {
                                crate::models::PdfError::EngineError(
                                    "Failed to communicate with engine".into(),
                                )
                            })?;

                        rx.await.map_err(|_| {
                            crate::models::PdfError::EngineError(
                                "Engine response channel closed".into(),
                            )
                        })?
                    } else {
                        Err(crate::models::PdfError::OpenFailed("Cancelled".into()))
                    }
                },
                Message::DocumentsMerged,
            )
        }
        Message::DocumentsMerged(res) => {
            match res {
                Ok(p) => app.status_message = Some(format!("Merged PDF saved to: {p}")),
                Err(e) => app.status_message = Some(format!("Merge failed: {e}")),
            }
            Task::none()
        }
        Message::SplitPDF(pages) => {
            let Some(tab) = app.current_tab() else {
                return Task::none();
            };
            let path = tab.path.to_string_lossy().to_string();
            let Some(engine) = &app.engine else {
                return Task::none();
            };
            let cmd_tx = engine.cmd_tx.clone();
            Task::perform(
                async move {
                    let folder = rfd::AsyncFileDialog::new()
                        .set_title("Select Output Folder for Split Pages")
                        .pick_folder()
                        .await;
                    if let Some(f) = folder {
                        let out_dir = f.path().to_string_lossy().to_string();
                        let (tx, rx) = tokio::sync::oneshot::channel();
                        let _ = cmd_tx.send(PdfCommand::Split(path, pages, out_dir, tx));
                        match rx.await {
                            Ok(res) => res,
                            Err(_) => Err(crate::models::PdfError::EngineDied),
                        }
                    } else {
                        Err(crate::models::PdfError::from("Cancelled"))
                    }
                },
                Message::PDFSplit,
            )
        }
        Message::PDFSplit(res) => {
            match res {
                Ok(paths) => app.status_message = Some(format!("Split into {} files", paths.len())),
                Err(e) => {
                    if e != "Cancelled" {
                        app.status_message = Some(format!("Split failed: {e}"));
                    }
                }
            }
            Task::none()
        }
        Message::LoadFormFields => {
            let (Some(tab), Some(engine)) = (app.current_tab(), &app.engine) else {
                return Task::none();
            };
            let path = tab.path.to_string_lossy().to_string();
            let cmd_tx = engine.cmd_tx.clone();
            Task::perform(
                async move {
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    cmd_tx
                        .send(PdfCommand::GetFormFields(path, tx))
                        .map_err(|_| crate::models::PdfError::EngineError("Engine died".into()))?;
                    rx.await.map_err(|_| {
                        crate::models::PdfError::EngineError(
                            "Engine response channel closed".into(),
                        )
                    })?
                },
                Message::FormFieldsLoaded,
            )
        }
        Message::FormFieldsLoaded(res) => {
            match res {
                Ok(fields) => app.form_fields = fields,
                Err(e) => app.status_message = Some(format!("Failed to load form fields: {e}")),
            }
            Task::none()
        }
        Message::FormFieldChanged(name, variant) => {
            if let Some(field) = app.form_fields.iter_mut().find(|f| f.name == name) {
                field.variant = variant;
            }
            Task::none()
        }
        Message::FillForm(fields) => {
            let (Some(tab), Some(engine)) = (app.current_tab(), &app.engine) else {
                return Task::none();
            };
            let path = tab.path.to_string_lossy().to_string();
            let cmd_tx = engine.cmd_tx.clone();
            Task::perform(
                async move {
                    let out = rfd::AsyncFileDialog::new()
                        .add_filter("PDF", &["pdf"])
                        .set_file_name("filled_form.pdf")
                        .save_file()
                        .await;
                    if let Some(f) = out {
                        let (tx, rx) = tokio::sync::oneshot::channel();
                        cmd_tx
                            .send(PdfCommand::FillForm(
                                path,
                                fields,
                                f.path().to_string_lossy().to_string(),
                                tx,
                            ))
                            .map_err(|_| {
                                crate::models::PdfError::EngineError("Engine died".into())
                            })?;
                        rx.await.map_err(|_| {
                            crate::models::PdfError::EngineError(
                                "Engine response channel closed".into(),
                            )
                        })?
                    } else {
                        Err(crate::models::PdfError::OpenFailed("Cancelled".into()))
                    }
                },
                Message::FormFilled,
            )
        }
        Message::FormFilled(res) => {
            match res {
                Ok(p) => app.status_message = Some(format!("Form saved to: {p}")),
                Err(e) => app.status_message = Some(format!("Form filling failed: {e}")),
            }
            Task::none()
        }
        Message::Print => {
            let Some(tab) = app.current_tab() else {
                return Task::none();
            };
            let path = tab.path.to_string_lossy().to_string();
            let Some(engine) = &app.engine else {
                return Task::none();
            };
            let cmd_tx = engine.cmd_tx.clone();
            Task::perform(
                async move {
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    let _ = cmd_tx.send(PdfCommand::PrintPdf(path, tx));
                    match rx.await {
                        Ok(res) => res,
                        Err(_) => Err(crate::models::PdfError::EngineDied),
                    }
                },
                Message::PrintDone,
            )
        }
        Message::PrintDone(res) => {
            match res {
                Ok(()) => app.status_message = Some("Document sent to printer".into()),
                Err(e) => app.status_message = Some(format!("Print failed: {e}")),
            }
            Task::none()
        }
        Message::AddWatermark(text) => {
            let Some(tab) = app.current_tab() else {
                return Task::none();
            };
            let path = tab.path.to_string_lossy().to_string();
            let Some(engine) = &app.engine else {
                return Task::none();
            };
            let cmd_tx = engine.cmd_tx.clone();
            Task::perform(
                async move {
                    let save = rfd::AsyncFileDialog::new()
                        .add_filter("PDF", &["pdf"])
                        .set_file_name("watermarked.pdf")
                        .set_title("Save Watermarked PDF")
                        .save_file()
                        .await;
                    match save {
                        Some(f) => {
                            let out = f.path().to_string_lossy().to_string();
                            let (tx, rx) = tokio::sync::oneshot::channel();
                            let _ =
                                cmd_tx.send(PdfCommand::AddWatermark(path, text, out.clone(), tx));
                            match rx.await {
                                Ok(Ok(path)) => Ok(path),
                                Ok(Err(e)) => Err(e),
                                Err(_) => Err(crate::models::PdfError::EngineDied),
                            }
                        }
                        None => Err(crate::models::PdfError::from("Cancelled")),
                    }
                },
                Message::WatermarkDone,
            )
        }
        Message::WatermarkDone(res) => {
            match res {
                Ok(path) => {
                    app.status_message = Some(format!("Watermarked PDF saved to: {path}"));
                }
                Err(e) => {
                    if e != "Cancelled" {
                        app.status_message = Some(format!("Watermark failed: {e}"));
                    }
                }
            }
            Task::none()
        }
        _ => Task::none(),
    }
}
