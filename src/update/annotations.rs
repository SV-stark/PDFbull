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
                if let Some((page_idx, start, current)) = tab.selection_drag {
                    let (boxes, text) = extract_selection_for_drag(tab, page_idx, start, current);
                    tab.selected_boxes = boxes;
                    tab.selected_text = text;
                }
            }
            Task::none()
        }
        Message::AnnotationDragEnd => {
            let ann_color = app.annotation_color.clone();
            let ann_thickness = app.annotation_thickness;
            let ann_text_size = app.annotation_text_size;
            let ann_text = app.annotation_text.clone();

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
                                    tab.annotations_dirty = true;
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
                                text: if ann_text.is_empty() {
                                    "Text Annotation".to_string()
                                } else {
                                    ann_text.clone()
                                },
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
                                comment: if ann_text.is_empty() {
                                    "Sticky Note".to_string()
                                } else {
                                    ann_text.clone()
                                },
                                color: "#ffeb3b".to_string(),
                            }
                        }
                    };

                    let (ann_x_vis, ann_y_vis, ann_w_vis, ann_h_vis) = match drag.kind {
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

                    let actual_page = tab
                        .page_mapping
                        .get(drag.page)
                        .copied()
                        .unwrap_or(drag.page);
                    let page_rotation = tab
                        .page_rotations
                        .get(&actual_page)
                        .copied()
                        .unwrap_or(tab.rotation);
                    let original_height = tab.page_heights.get(drag.page).copied().unwrap_or(800.0);

                    let (ann_x, ann_y, ann_w, ann_h) = crate::models::unrotate_coords(
                        ann_x_vis,
                        ann_y_vis,
                        ann_w_vis,
                        ann_h_vis,
                        tab.page_width,
                        original_height,
                        page_rotation,
                    );

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
                    tab.annotations_dirty = true;
                }
            } else if let Some(tab) = app.current_tab_mut()
                && let Some((page_idx, start, current)) = tab.selection_drag.take()
            {
                let (boxes, text) = extract_selection_for_drag(tab, page_idx, start, current);
                tab.selected_boxes = boxes;
                tab.selected_text = text.clone();
                if let Some(text) = text {
                    let mut clipboard = arboard::Clipboard::new().ok();
                    if let Some(cb) = &mut clipboard {
                        let _ = cb.set_text(text);
                    }
                    app.status_message = Some(
                        "Text copied to clipboard! (Ctrl+C to copy, H to highlight)".to_string(),
                    );
                }
            }
            Task::none()
        }
        Message::HighlightSelection => {
            let color = app.annotation_color.clone();
            if let Some(tab) = app.current_tab_mut()
                && !tab.selected_boxes.is_empty()
            {
                for (page, x, y, w, h) in tab.selected_boxes.clone() {
                    let id = crate::models::next_annotation_id();
                    let ann = crate::models::Annotation {
                        id,
                        page,
                        style: crate::models::AnnotationStyle::Highlight {
                            color: color.clone(),
                        },
                        x,
                        y,
                        width: w,
                        height: h,
                    };
                    tab.undo_stack
                        .push(crate::models::UndoableAction::AddAnnotation(ann.clone()));
                    tab.annotations.push(ann);
                }
                tab.redo_stack.clear();
                tab.annotations_dirty = true;
                tab.selected_text = None;
                tab.selected_boxes.clear();
                app.status_message =
                    Some("Highlighted selected text! Press Ctrl+S to save to PDF.".to_string());
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
                tab.annotations_dirty = true;
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
                        if idx <= tab.annotations.len() {
                            tab.annotations.insert(idx, ann);
                        } else {
                            tab.annotations.push(ann);
                        }
                    }
                }
                tab.annotations_dirty = true;
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
                tab.annotations_dirty = true;
            }
            Task::none()
        }
        Message::SaveAnnotations => {
            if let Some(tab) = app.current_tab() {
                let doc_id = tab.id;
                let annotations = tab.annotations.clone();
                let Some(engine) = &app.engine else {
                    return Task::none();
                };
                let cmd_tx = engine.cmd_tx.clone();
                Task::perform(
                    async move {
                        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                        let _ = cmd_tx
                            .send(crate::commands::PdfCommand::SaveAnnotations(
                                doc_id,
                                annotations,
                                resp_tx,
                            ))
                            .await;
                        resp_rx.await.unwrap_or_else(|_| {
                            Err(crate::models::PdfError::IoError(
                                "Channel closed".to_string(),
                            ))
                        })
                    },
                    Message::AnnotationsSaved,
                )
            } else {
                Task::none()
            }
        }
        Message::AnnotationsSaved(res) => {
            match res {
                Ok(path) => {
                    if let Some(tab) = app.current_tab_mut() {
                        tab.annotations_dirty = false;
                    }
                    app.status_message = Some(format!("Annotations saved to: {path}"));
                }
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
        Message::EditAnnotationText(idx, new_text) => {
            if let Some(tab) = app.current_tab_mut()
                && let Some(ann) = tab.annotations.get_mut(idx)
            {
                match &mut ann.style {
                    crate::models::AnnotationStyle::Text { text, .. } => {
                        *text = new_text;
                        tab.annotations_dirty = true;
                    }
                    crate::models::AnnotationStyle::StickyNote { comment, .. } => {
                        *comment = new_text;
                        tab.annotations_dirty = true;
                    }
                    _ => {}
                }
            }
            Task::none()
        }
        _ => Task::none(),
    }
}

pub type SelectionBox = (usize, f32, f32, f32, f32);

pub fn extract_selection_for_drag(
    tab: &crate::models::DocumentTab,
    page_idx: usize,
    start: (f32, f32),
    current: (f32, f32),
) -> (Vec<SelectionBox>, Option<String>) {
    let actual_page = tab.page_mapping.get(page_idx).copied().unwrap_or(page_idx);
    let page_rotation = tab
        .page_rotations
        .get(&actual_page)
        .copied()
        .unwrap_or(tab.rotation);
    let original_height = tab.page_heights.get(page_idx).copied().unwrap_or(800.0);

    let vx = start.0.min(current.0);
    let vy = start.1.min(current.1);
    let vw = (current.0 - start.0).abs();
    let vh = (current.1 - start.1).abs();

    if vw < 3.0 && vh < 3.0 {
        return (Vec::new(), None);
    }

    let (ux, uy, uw, uh) = crate::models::unrotate_coords(
        vx,
        vy,
        vw,
        vh,
        tab.page_width,
        original_height,
        page_rotation,
    );

    let x1 = ux;
    let x2 = ux + uw;
    let y1 = uy;
    let y2 = uy + uh;

    let mut selected_words = Vec::new();
    if let Some(words) = tab.view_state.text_layers.get(&page_idx) {
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

    if selected_words.is_empty() {
        return (Vec::new(), None);
    }

    selected_words.sort_by(|a, b| {
        let threshold = (a.height.min(b.height) * 0.6).max(4.0);
        let y_diff = (a.y - b.y).abs();
        if y_diff < threshold {
            a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal)
        } else {
            a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal)
        }
    });

    // Group words into lines
    let mut line_groups: Vec<Vec<&crate::models::TextItem>> = Vec::new();
    for word in &selected_words {
        if let Some(cur_line) = line_groups.last_mut() {
            let first_y = cur_line[0].y;
            let threshold = (cur_line[0].height.max(word.height) * 0.6).max(4.0);
            if (word.y - first_y).abs() < threshold {
                cur_line.push(word);
                continue;
            }
        }
        line_groups.push(vec![word]);
    }

    // Sort horizontally within each line
    for line in &mut line_groups {
        line.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
    }

    let text: String = line_groups
        .iter()
        .map(|line| {
            line.iter()
                .map(|w| w.text.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Produce merged bounding box per line segment for clean highlight rendering
    let mut boxes = Vec::new();
    for line in line_groups {
        if line.is_empty() {
            continue;
        }
        let mut min_x = line[0].x;
        let mut min_y = line[0].y;
        let mut max_x = line[0].x + line[0].width;
        let mut max_y = line[0].y + line[0].height;

        for w in &line[1..] {
            min_x = min_x.min(w.x);
            min_y = min_y.min(w.y);
            max_x = max_x.max(w.x + w.width);
            max_y = max_y.max(w.y + w.height);
        }

        boxes.push((page_idx, min_x, min_y, max_x - min_x, max_y - min_y));
    }

    (boxes, Some(text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{DocumentTab, TextItem};

    fn make_test_tab() -> DocumentTab {
        let mut tab = DocumentTab::new(std::path::PathBuf::from("test.pdf"));
        tab.total_pages = 2;
        tab.page_width = 600.0;
        tab.page_heights = vec![800.0, 800.0];

        // Add mock text items to page 0:
        // Line 1 (y = 100): "Hello" (x=50..100), "World" (x=110..170)
        // Line 2 (y = 130): "PDFbull" (x=50..120), "Reader" (x=130..190)
        let page_0_words = vec![
            TextItem {
                text: "Hello".to_string(),
                x: 50.0,
                y: 100.0,
                width: 50.0,
                height: 16.0,
            },
            TextItem {
                text: "World".to_string(),
                x: 110.0,
                y: 100.0,
                width: 60.0,
                height: 16.0,
            },
            TextItem {
                text: "PDFbull".to_string(),
                x: 50.0,
                y: 130.0,
                width: 70.0,
                height: 16.0,
            },
            TextItem {
                text: "Reader".to_string(),
                x: 130.0,
                y: 130.0,
                width: 60.0,
                height: 16.0,
            },
        ];

        tab.view_state.text_layers.insert(0, page_0_words);
        tab
    }

    #[test]
    fn test_drag_selection_single_word() {
        let tab = make_test_tab();
        // Drag over "Hello" (x: 40..105, y: 95..120)
        let (boxes, text) = extract_selection_for_drag(&tab, 0, (40.0, 95.0), (105.0, 120.0));
        assert_eq!(text, Some("Hello".to_string()));
        assert_eq!(boxes.len(), 1);
        assert_eq!(boxes[0], (0, 50.0, 100.0, 50.0, 16.0));
    }

    #[test]
    fn test_drag_selection_full_line() {
        let tab = make_test_tab();
        // Drag over both "Hello" and "World" on line 1
        let (boxes, text) = extract_selection_for_drag(&tab, 0, (40.0, 95.0), (180.0, 120.0));
        assert_eq!(text, Some("Hello World".to_string()));
        assert_eq!(boxes.len(), 1);
        // Box should merge horizontally: min_x=50.0, max_x=170.0 -> width=120.0
        assert_eq!(boxes[0], (0, 50.0, 100.0, 120.0, 16.0));
    }

    #[test]
    fn test_drag_selection_multiline() {
        let tab = make_test_tab();
        // Drag across both Line 1 and Line 2
        let (boxes, text) = extract_selection_for_drag(&tab, 0, (40.0, 90.0), (200.0, 150.0));
        assert_eq!(text, Some("Hello World\nPDFbull Reader".to_string()));
        assert_eq!(boxes.len(), 2);
        // Line 1 box
        assert_eq!(boxes[0], (0, 50.0, 100.0, 120.0, 16.0));
        // Line 2 box: min_x=50.0, max_x=190.0 -> width=140.0
        assert_eq!(boxes[1], (0, 50.0, 130.0, 140.0, 16.0));
    }

    #[test]
    fn test_click_to_clear_subthreshold() {
        let tab = make_test_tab();
        // Micro-movement / single click (< 3px)
        let (boxes, text) = extract_selection_for_drag(&tab, 0, (60.0, 105.0), (61.0, 106.0));
        assert!(boxes.is_empty());
        assert_eq!(text, None);
    }

    #[test]
    fn test_drag_empty_area() {
        let tab = make_test_tab();
        // Drag over empty space (x: 400..500, y: 400..500)
        let (boxes, text) = extract_selection_for_drag(&tab, 0, (400.0, 400.0), (500.0, 500.0));
        assert!(boxes.is_empty());
        assert_eq!(text, None);
    }

    #[test]
    fn test_drag_selection_with_page_rotation() {
        let mut tab = make_test_tab();
        // Set rotation to 180 degrees
        tab.rotation = 180;
        // Unrotated coords for "Hello" are x: 50..100, y: 100..116.
        // At 180 deg (pw=600, ph=800):
        // rx = 600 - (100) = 500
        // ry = 800 - (116) = 684
        // Drag over this rotated area in visual space:
        let (boxes, text) = extract_selection_for_drag(&tab, 0, (490.0, 680.0), (560.0, 710.0));
        assert_eq!(text, Some("Hello".to_string()));
        assert_eq!(boxes.len(), 1);
        assert_eq!(boxes[0], (0, 50.0, 100.0, 50.0, 16.0));
    }
}
