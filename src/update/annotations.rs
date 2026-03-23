use crate::app::PdfBullApp;
use crate::message::Message;
use iced::Task;

pub fn handle_annotation_message(app: &mut PdfBullApp, message: Message) -> Task<Message> {
    match message {
        Message::SetAnnotationMode(mode) => {
            app.annotation_mode = mode;
            if app.annotation_mode.is_none() {
                app.annotation_drag = None;
            }
            Task::none()
        }
        Message::AnnotationDragStart { page, x, y } => {
            if let Some(kind) = &app.annotation_mode {
                app.annotation_drag = Some(crate::models::AnnotationDrag {
                    page,
                    start: (x, y),
                    current: (x, y),
                    kind: kind.clone(),
                });
            }
            Task::none()
        }
        Message::AnnotationDragUpdate { x, y } => {
            if let Some(drag) = &mut app.annotation_drag {
                drag.current = (x, y);
            }
            Task::none()
        }
        Message::AnnotationDragEnd => {
            if let Some(drag) = app.annotation_drag.take() {
                if let Some(tab) = app.current_tab_mut() {
                    let zoom = tab.zoom;
                    let min_x = drag.start.0.min(drag.current.0);
                    let min_y = drag.start.1.min(drag.current.1);
                    let w = (drag.start.0 - drag.current.0).abs();
                    let h = (drag.start.1 - drag.current.1).abs();

                    if (w / zoom) > 5.0 && (h / zoom) > 5.0 {
                        let id = crate::models::next_annotation_id();
                        let style = match drag.kind {
                            crate::models::PendingAnnotationKind::Highlight => {
                                crate::models::AnnotationStyle::Highlight {
                                    color: "#FFFF00".to_string(),
                                }
                            }
                            crate::models::PendingAnnotationKind::Rectangle => {
                                crate::models::AnnotationStyle::Rectangle {
                                    color: "#FF0000".to_string(),
                                    thickness: 2.0,
                                    fill: false,
                                }
                            }
                            crate::models::PendingAnnotationKind::Redact => {
                                crate::models::AnnotationStyle::Redact {
                                    color: "#000000".to_string(),
                                }
                            }
                        };

                        let ann = crate::models::Annotation {
                            id,
                            page: drag.page,
                            style,
                            x: min_x / zoom,
                            y: min_y / zoom,
                            width: w / zoom,
                            height: h / zoom,
                        };

                        tab.undo_stack
                            .push(crate::models::UndoableAction::AddAnnotation(ann.clone()));
                        tab.redo_stack.clear();
                        tab.annotations.push(ann);
                    }
                }
            }
            Task::none()
        }
        Message::DeleteAnnotation(idx) => {
            if let Some(tab) = app.current_tab_mut() {
                if idx < tab.annotations.len() {
                    let ann = tab.annotations.remove(idx);
                    tab.undo_stack
                        .push(crate::models::UndoableAction::DeleteAnnotation(idx, ann));
                    tab.redo_stack.clear();
                }
            }
            Task::none()
        }
        Message::Undo => {
            if let Some(tab) = app.current_tab_mut() {
                if let Some(action) = tab.undo_stack.pop() {
                    match action {
                        crate::models::UndoableAction::AddAnnotation(ann) => {
                            tab.redo_stack
                                .push(crate::models::UndoableAction::AddAnnotation(ann.clone()));
                            tab.annotations.retain(|a| a.id != ann.id);
                        }
                        crate::models::UndoableAction::DeleteAnnotation(idx, ann) => {
                            tab.redo_stack
                                .push(crate::models::UndoableAction::DeleteAnnotation(
                                    idx,
                                    ann.clone(),
                                ));
                            tab.annotations.insert(idx.min(tab.annotations.len()), ann);
                        }
                    }
                }
            }
            Task::none()
        }
        Message::Redo => {
            if let Some(tab) = app.current_tab_mut() {
                if let Some(action) = tab.redo_stack.pop() {
                    match action {
                        crate::models::UndoableAction::AddAnnotation(ann) => {
                            tab.undo_stack
                                .push(crate::models::UndoableAction::AddAnnotation(ann.clone()));
                            tab.annotations.push(ann);
                        }
                        crate::models::UndoableAction::DeleteAnnotation(idx, ann) => {
                            tab.undo_stack
                                .push(crate::models::UndoableAction::DeleteAnnotation(
                                    idx,
                                    ann.clone(),
                                ));
                            tab.annotations.retain(|a| a.id != ann.id);
                        }
                    }
                }
            }
            Task::none()
        }
        Message::SaveAnnotations => {
            let (doc_id, annotations, pdf_path) = match app.current_tab() {
                Some(t) if !t.annotations.is_empty() => (
                    t.id,
                    t.annotations.clone(),
                    t.path.to_string_lossy().to_string(),
                ),
                _ => {
                    tracing::warn!("No annotations to save");
                    return Task::none();
                }
            };

            let Some(engine) = &app.engine else {
                return Task::none();
            };

            let cmd_tx = engine.cmd_tx.clone();
            Task::perform(
                async move {
                    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                    let _ = cmd_tx.send(crate::commands::PdfCommand::ExportPdf(
                        doc_id,
                        pdf_path,
                        annotations,
                        resp_tx,
                    ));
                    match resp_rx.await {
                        Ok(Ok(path)) => Ok(path),
                        Ok(Err(e)) => Err(e),
                        Err(_) => Err(crate::models::PdfError::from("Engine died")),
                    }
                },
                Message::AnnotationsSaved,
            )
        }
        Message::AnnotationsSaved(result) => {
            match result {
                Ok(path) => app.status_message = Some(format!("Annotations saved to: {path}")),
                Err(e) => {
                    tracing::error!("Error saving annotations: {e}");
                    app.status_message = Some(format!("Error saving annotations: {e}"));
                }
            }
            Task::none()
        }
        Message::AnnotationsLoaded(doc_id, annotations) => {
            if let Some(tab) = app.tabs.iter_mut().find(|t| t.id == doc_id) {
                tab.annotations = annotations;
            }
            app.render_visible_pages()
        }
        _ => Task::none(),
    }
}
