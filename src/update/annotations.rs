use crate::app::PdfBullApp;
use crate::message::Message;
use iced::Task;

#[allow(clippy::suboptimal_flops, clippy::similar_names)]
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
            } else if let Some(tab) = app.current_tab_mut() {
                tab.selected_text = None;
                tab.selected_boxes.clear();
                let zoom = tab.zoom;
                tab.selection_drag = Some((page, (x / zoom, y / zoom), (x / zoom, y / zoom)));
            }
            Task::none()
        }
        Message::AnnotationDragUpdate { x, y } => {
            if let Some(drag) = &mut app.annotation_drag {
                drag.current = (x, y);
            } else if let Some(tab) = app.current_tab_mut() {
                if let Some((_, _, current)) = &mut tab.selection_drag {
                    let zoom = tab.zoom;
                    *current = (x / zoom, y / zoom);
                }
            }
            Task::none()
        }
        Message::AnnotationDragEnd => {
            let ann_color = app.annotation_color.clone();
            let ann_thickness = app.annotation_thickness;
            let ann_text_size = app.annotation_text_size;

            let is_stamp = app.signature_stamp_active;
            let sig_strokes = app.saved_signature.clone();

            if let Some(drag) = app.annotation_drag.take() {
                if is_stamp && let Some(sig_lines) = sig_strokes {
                    if let Some(tab) = app.current_tab_mut() {
                        let zoom = tab.zoom;
                        let click_x = drag.start.0 / zoom;
                        let click_y = drag.start.1 / zoom;

                        let mut min_x = f32::MAX;
                        let mut max_x = f32::MIN;
                        let mut min_y = f32::MAX;
                        let mut max_y = f32::MIN;

                        for stroke in &sig_lines {
                            for &(sx, sy) in stroke {
                                if sx < min_x {
                                    min_x = sx;
                                }
                                if sx > max_x {
                                    max_x = sx;
                                }
                                if sy < min_y {
                                    min_y = sy;
                                }
                                if sy > max_y {
                                    max_y = sy;
                                }
                            }
                        }

                        let w = max_x - min_x;
                        let h = max_y - min_y;

                        if w > 1.0 && h > 1.0 {
                            let stamp_w = 120.0;
                            let stamp_h = stamp_w * (h / w);

                            let offset_x = click_x - stamp_w / 2.0;
                            let offset_y = click_y - stamp_h / 2.0;

                            for stroke in &sig_lines {
                                for window in stroke.windows(2) {
                                    let p1 = window[0];
                                    let p2 = window[1];

                                    let rel_x1 = (p1.0 - min_x) / w;
                                    let rel_y1 = (p1.1 - min_y) / h;
                                    let rel_x2 = (p2.0 - min_x) / w;
                                    let rel_y2 = (p2.1 - min_y) / h;

                                    let sx1 = offset_x + rel_x1 * stamp_w;
                                    let sy1 = offset_y + rel_y1 * stamp_h;
                                    let sx2 = offset_x + rel_x2 * stamp_w;
                                    let sy2 = offset_y + rel_y2 * stamp_h;

                                    let id = crate::models::next_annotation_id();
                                    let line_ann = crate::models::Annotation {
                                        id,
                                        page: drag.page,
                                        style: crate::models::AnnotationStyle::Line {
                                            color: "#2c3e50".to_string(), // Dark ink color
                                            thickness: 2.0,
                                        },
                                        x: sx1,
                                        y: sy1,
                                        width: sx2 - sx1,
                                        height: sy2 - sy1,
                                    };
                                    tab.annotations.push(line_ann);
                                }
                            }
                        }
                    }

                    app.signature_stamp_active = false;
                    app.annotation_mode = None;
                    return app.render_visible_pages();
                }

                let Some(tab) = app.current_tab_mut() else {
                    return Task::none();
                };

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
                                color: "#000000".to_string(),
                            }
                        }
                        crate::models::PendingAnnotationKind::Text => {
                            crate::models::AnnotationStyle::Text {
                                text: "Text Annotation".to_string(),
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
                                comment: "Sticky Note".to_string(),
                                color: "#ffeb3b".to_string(),
                            }
                        }
                    };

                    let (ann_x, ann_y, ann_w, ann_h) = match drag.kind {
                        crate::models::PendingAnnotationKind::Line
                        | crate::models::PendingAnnotationKind::Arrow => {
                            (start_x / zoom, start_y / zoom, dx / zoom, dy / zoom)
                        }
                        crate::models::PendingAnnotationKind::Text
                        | crate::models::PendingAnnotationKind::StickyNote => {
                            let click_x = start_x / zoom;
                            let click_y = start_y / zoom;
                            (click_x, click_y, 120.0, 24.0)
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
            } else if let Some(tab) = app.current_tab_mut() {
                if let Some((page_idx, start, current)) = tab.selection_drag.take() {
                    let mut selected_words = Vec::new();
                    if let Some(words) = tab.view_state.text_layers.get(&page_idx) {
                        let x1 = start.0.min(current.0);
                        let x2 = start.0.max(current.0);
                        let y1 = start.1.min(current.1);
                        let y2 = start.1.max(current.1);

                        for word in words {
                            let wx1 = word.x;
                            let wx2 = word.x + word.width;
                            let wy1 = word.y;
                            let wy2 = word.y + word.height;

                            let overlap_x = x1 < wx2 && x2 > wx1;
                            let overlap_y = y1 < wy2 && y2 > wy1;

                            if overlap_x && overlap_y {
                                selected_words.push(word.clone());
                            }
                        }
                    }
                    if !selected_words.is_empty() {
                        selected_words.sort_by(|a, b| {
                            let y_diff = (a.y - b.y).abs();
                            if y_diff < 5.0 {
                                a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal)
                            } else {
                                a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal)
                            }
                        });
                        let text: String = selected_words
                            .iter()
                            .map(|w| w.text.as_str())
                            .collect::<Vec<_>>()
                            .join(" ");
                        tab.selected_boxes = selected_words
                            .iter()
                            .map(|w| (w.x, w.y, w.width, w.height))
                            .collect();
                        tab.selected_text = Some(text.clone());

                        let mut clipboard = arboard::Clipboard::new().ok();
                        if let Some(cb) = &mut clipboard {
                            let _ = cb.set_text(text);
                        }
                        app.status_message = Some("Text copied to clipboard!".to_string());
                    }
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
