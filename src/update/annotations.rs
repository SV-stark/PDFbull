use crate::app::PdfBullApp;
use crate::message::Message;
use iced::Task;

#[allow(clippy::suboptimal_flops)]
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
                    kind: *kind,
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
            let ann_color = app.annotation_color.clone();
            let ann_thickness = app.annotation_thickness;
            let ann_text_size = app.annotation_text_size;

            if let Some(drag) = app.annotation_drag.take()
                && let Some(tab) = app.current_tab_mut()
            {
                let zoom = tab.zoom;
                let start_x = drag.start.0;
                let start_y = drag.start.1;
                let curr_x = drag.current.0;
                let curr_y = drag.current.1;

                let dx = curr_x - start_x;
                let dy = curr_y - start_y;
                let dist = dx.hypot(dy);

                let is_sticky = drag.kind == crate::models::PendingAnnotationKind::StickyNote;
                let is_valid = dist > 5.0 || is_sticky;

                if is_valid {
                    let id = crate::models::next_annotation_id();
                    let style = match drag.kind {
                        crate::models::PendingAnnotationKind::Highlight => {
                            crate::models::AnnotationStyle::Highlight {
                                color: ann_color.clone(),
                            }
                        }
                        crate::models::PendingAnnotationKind::Rectangle => {
                            crate::models::AnnotationStyle::Rectangle {
                                color: ann_color.clone(),
                                thickness: ann_thickness,
                                fill: false,
                            }
                        }
                        crate::models::PendingAnnotationKind::Redact => {
                            crate::models::AnnotationStyle::Redact {
                                color: ann_color.clone(),
                            }
                        }
                        crate::models::PendingAnnotationKind::Text => {
                            crate::models::AnnotationStyle::Text {
                                text: "New Text".to_string(),
                                color: ann_color.clone(),
                                font_size: ann_text_size as u32,
                            }
                        }
                        crate::models::PendingAnnotationKind::Circle => {
                            crate::models::AnnotationStyle::Circle {
                                color: ann_color.clone(),
                                thickness: ann_thickness,
                                fill: false,
                            }
                        }
                        crate::models::PendingAnnotationKind::Line => {
                            crate::models::AnnotationStyle::Line {
                                color: ann_color.clone(),
                                thickness: ann_thickness,
                            }
                        }
                        crate::models::PendingAnnotationKind::Arrow => {
                            crate::models::AnnotationStyle::Arrow {
                                color: ann_color.clone(),
                                thickness: ann_thickness,
                            }
                        }
                        crate::models::PendingAnnotationKind::StickyNote => {
                            crate::models::AnnotationStyle::StickyNote {
                                comment: "New sticky note comment".to_string(),
                                color: ann_color.clone(),
                            }
                        }
                    };

                    let (ann_x, ann_y, ann_w, ann_h) = match drag.kind {
                        crate::models::PendingAnnotationKind::Line
                        | crate::models::PendingAnnotationKind::Arrow => {
                            (start_x / zoom, start_y / zoom, dx / zoom, dy / zoom)
                        }
                        crate::models::PendingAnnotationKind::StickyNote if dist <= 5.0 => {
                            // Centered 24x24 box at click point
                            (
                                (start_x - 12.0 * zoom) / zoom,
                                (start_y - 12.0 * zoom) / zoom,
                                24.0,
                                24.0,
                            )
                        }
                        _ => {
                            let min_x = start_x.min(curr_x);
                            let min_y = start_y.min(curr_y);
                            let w = (start_x - curr_x).abs();
                            let h = (start_y - curr_y).abs();
                            (min_x / zoom, min_y / zoom, w / zoom, h / zoom)
                        }
                    };

                    let ann = crate::models::Annotation {
                        id,
                        page: drag.page,
                        style,
                        x: ann_x,
                        y: ann_y,
                        width: ann_w,
                        height: ann_h,
                    };

                    tab.undo_stack
                        .push(crate::models::UndoableAction::AddAnnotation(ann.clone()));
                    tab.redo_stack.clear();
                    tab.annotations.push(ann);
                }
            }
            Task::none()
        }
        Message::DeleteAnnotation(idx) => {
            if let Some(tab) = app.current_tab_mut()
                && idx < tab.annotations.len()
            {
                let ann = tab.annotations.remove(idx);
                tab.undo_stack
                    .push(crate::models::UndoableAction::DeleteAnnotation(idx, ann));
                tab.redo_stack.clear();
            }
            Task::none()
        }
        Message::Undo => {
            if let Some(tab) = app.current_tab_mut()
                && let Some(action) = tab.undo_stack.pop()
            {
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
            Task::none()
        }
        Message::Redo => {
            if let Some(tab) = app.current_tab_mut()
                && let Some(action) = tab.redo_stack.pop()
            {
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
                        Err(_) => Err(crate::models::PdfError::EngineDied),
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
