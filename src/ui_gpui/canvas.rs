use super::ribbon::PageLayoutMode;
use gpui_kit::base::StyledExt;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::gpui::{BorderStyle, fill, outline};
use gpui_kit::*;

/// Converts an RGBA pixel buffer (potentially with tiny-skia premultiplied alpha)
/// to BGRA format as expected by GPUI Direct3D 11 textures.
pub fn convert_rgba_to_bgra(data: &mut [u8]) {
    for pixel in data.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
        let a = pixel[3];
        if a > 0 && a < 255 {
            let alpha = a as f32 / 255.0;
            pixel[0] = ((pixel[0] as f32 / alpha).min(255.0)) as u8;
            pixel[1] = ((pixel[1] as f32 / alpha).min(255.0)) as u8;
            pixel[2] = ((pixel[2] as f32 / alpha).min(255.0)) as u8;
        }
    }
}

pub struct DocumentViewport {
    pub zoom: f32,
    pub rotation: u16,
    pub current_page: usize,
    pub total_pages: usize,
    pub page_width: f32,
    pub page_height: f32,
    pub layout_mode: PageLayoutMode,
    pub standalone_cover: bool,
    pub is_selecting: bool,
    pub selection_page: Option<usize>,
    pub selection_start: Option<Point<Pixels>>,
    pub selection_end: Option<Point<Pixels>>,
    pub selected_text: Option<String>,
    pub text_cache: std::collections::HashMap<usize, Vec<crate::models::TextItem>>,
    pub rendered_pages: std::collections::HashMap<usize, std::sync::Arc<RenderImage>>,
    pub rendered_zoom: std::collections::HashMap<usize, f32>,
    pub scroll_handle: ScrollHandle,
    pub page_origins:
        std::sync::Arc<std::sync::RwLock<std::collections::HashMap<usize, Point<Pixels>>>>,
    pub highlight_color: Option<Hsla>,
    pub annotations: Vec<crate::models::Annotation>,
    pub search_highlights: std::collections::HashMap<usize, Vec<(f32, f32, f32, f32)>>,
}

impl Default for DocumentViewport {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentViewport {
    pub fn new() -> Self {
        Self {
            zoom: 1.0,
            rotation: 0,
            current_page: 0,
            total_pages: 1,
            page_width: 595.0,  // Standard A4 width in pt
            page_height: 842.0, // Standard A4 height in pt
            layout_mode: PageLayoutMode::Continuous,
            standalone_cover: false,
            is_selecting: false,
            selection_page: None,
            selection_start: None,
            selection_end: None,
            selected_text: None,
            text_cache: std::collections::HashMap::new(),
            rendered_pages: std::collections::HashMap::new(),
            rendered_zoom: std::collections::HashMap::new(),
            scroll_handle: ScrollHandle::new(),
            page_origins: std::sync::Arc::new(std::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
            highlight_color: None,
            annotations: Vec::new(),
            search_highlights: std::collections::HashMap::new(),
        }
    }

    fn render_page_card<V: 'static>(
        &self,
        cx: &mut Context<V>,
        page_idx: usize,
        on_page_click: impl Fn(&mut V, usize, &mut Window, &mut Context<V>) + 'static + Copy,
        on_drag_start: impl Fn(&mut V, usize, Point<Pixels>, &mut Window, &mut Context<V>)
        + 'static
        + Copy,
        on_drag_move: impl Fn(&mut V, usize, Point<Pixels>, &mut Window, &mut Context<V>)
        + 'static
        + Copy,
        on_drag_end: impl Fn(&mut V, usize, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> AnyElement {
        let primary = cx.theme().primary;
        let border = cx.theme().border;
        let muted_fg = cx.theme().muted_foreground;
        let scaled_w = px(self.page_width * self.zoom);
        let scaled_h = px(self.page_height * self.zoom);
        let is_active = page_idx == self.current_page;
        let total = self.total_pages;

        let down_listener = cx.listener(move |v, event: &MouseDownEvent, w, cx| {
            on_drag_start(v, page_idx, event.position, w, cx);
        });
        let move_listener = cx.listener(move |v, event: &MouseMoveEvent, w, cx| {
            on_drag_move(v, page_idx, event.position, w, cx);
        });
        let up_listener = cx.listener(move |v, _event: &MouseUpEvent, w, cx| {
            on_drag_end(v, page_idx, w, cx);
        });
        let click_listener = cx.listener(move |v, _, w, cx| on_page_click(v, page_idx, w, cx));

        let is_sel_page = self.selection_page == Some(page_idx);
        let sel_start = self.selection_start;
        let sel_end = self.selection_end;
        let is_selecting = self.is_selecting;
        let text_items = self.text_cache.get(&page_idx).cloned();
        let page_origins = self.page_origins.clone();
        let zoom = self.zoom;
        let hl_color = self.highlight_color;
        let page_annotations: Vec<crate::models::Annotation> = self
            .annotations
            .iter()
            .filter(|a| a.page == page_idx)
            .cloned()
            .collect();
        let page_search_matches = self
            .search_highlights
            .get(&page_idx)
            .cloned()
            .unwrap_or_default();

        let selection_overlay = canvas(
            move |bounds, _, _| bounds,
            move |_bounds, card_bounds, window, _| {
                // Record the actual window origin of this page card for coordinate mapping
                if let Ok(mut origins) = page_origins.write() {
                    origins.insert(page_idx, card_bounds.origin);
                }

                // 0. Paint search matches on this page
                let search_fill = hsla(45.0 / 360.0, 1.0, 0.5, 0.45);
                for (mx, my, mw, mh) in &page_search_matches {
                    let quad = Bounds {
                        origin: Point::new(
                            card_bounds.origin.x + px(mx * zoom),
                            card_bounds.origin.y + px(my * zoom),
                        ),
                        size: Size {
                            width: px(mw * zoom),
                            height: px(mh * zoom),
                        },
                    };
                    let clipped = quad.intersect(&card_bounds);
                    if clipped.size.width > px(1.0) && clipped.size.height > px(1.0) {
                        window.paint_quad(fill(clipped, search_fill));
                    }
                }

                // 1. Paint saved permanent annotations on this page
                for ann in &page_annotations {
                    let ann_quad = Bounds {
                        origin: Point::new(
                            card_bounds.origin.x + px(ann.x * zoom),
                            card_bounds.origin.y + px(ann.y * zoom),
                        ),
                        size: Size {
                            width: px(ann.width * zoom),
                            height: px(ann.height * zoom),
                        },
                    };
                    let clipped_ann = ann_quad.intersect(&card_bounds);
                    if clipped_ann.size.width > px(1.0) && clipped_ann.size.height > px(1.0) {
                        match &ann.style {
                            crate::models::AnnotationStyle::Highlight { .. } => {
                                window.paint_quad(fill(
                                    clipped_ann,
                                    hsla(50.0 / 360.0, 1.0, 0.5, 0.45),
                                ));
                            }
                            crate::models::AnnotationStyle::Redact { .. } => {
                                window.paint_quad(fill(clipped_ann, hsla(0.0, 0.0, 0.05, 0.98)));
                            }
                            crate::models::AnnotationStyle::Rectangle { .. } => {
                                let stroke_c = hsla(215.0 / 360.0, 0.9, 0.5, 0.9);
                                let t = px(2.0);
                                window.paint_quad(fill(
                                    Bounds {
                                        origin: clipped_ann.origin,
                                        size: Size {
                                            width: clipped_ann.size.width,
                                            height: t,
                                        },
                                    },
                                    stroke_c,
                                ));
                                window.paint_quad(fill(
                                    Bounds {
                                        origin: Point::new(
                                            clipped_ann.origin.x,
                                            clipped_ann.origin.y + clipped_ann.size.height - t,
                                        ),
                                        size: Size {
                                            width: clipped_ann.size.width,
                                            height: t,
                                        },
                                    },
                                    stroke_c,
                                ));
                                window.paint_quad(fill(
                                    Bounds {
                                        origin: clipped_ann.origin,
                                        size: Size {
                                            width: t,
                                            height: clipped_ann.size.height,
                                        },
                                    },
                                    stroke_c,
                                ));
                                window.paint_quad(fill(
                                    Bounds {
                                        origin: Point::new(
                                            clipped_ann.origin.x + clipped_ann.size.width - t,
                                            clipped_ann.origin.y,
                                        ),
                                        size: Size {
                                            width: t,
                                            height: clipped_ann.size.height,
                                        },
                                    },
                                    stroke_c,
                                ));
                            }
                            crate::models::AnnotationStyle::Circle { .. } => {
                                let stroke_c = hsla(140.0 / 360.0, 0.8, 0.45, 0.9);
                                let t = px(2.0);
                                window.paint_quad(fill(
                                    Bounds {
                                        origin: clipped_ann.origin,
                                        size: Size {
                                            width: clipped_ann.size.width,
                                            height: t,
                                        },
                                    },
                                    stroke_c,
                                ));
                                window.paint_quad(fill(
                                    Bounds {
                                        origin: Point::new(
                                            clipped_ann.origin.x,
                                            clipped_ann.origin.y + clipped_ann.size.height - t,
                                        ),
                                        size: Size {
                                            width: clipped_ann.size.width,
                                            height: t,
                                        },
                                    },
                                    stroke_c,
                                ));
                                window.paint_quad(fill(
                                    Bounds {
                                        origin: clipped_ann.origin,
                                        size: Size {
                                            width: t,
                                            height: clipped_ann.size.height,
                                        },
                                    },
                                    stroke_c,
                                ));
                                window.paint_quad(fill(
                                    Bounds {
                                        origin: Point::new(
                                            clipped_ann.origin.x + clipped_ann.size.width - t,
                                            clipped_ann.origin.y,
                                        ),
                                        size: Size {
                                            width: t,
                                            height: clipped_ann.size.height,
                                        },
                                    },
                                    stroke_c,
                                ));
                            }
                            crate::models::AnnotationStyle::Line { .. }
                            | crate::models::AnnotationStyle::Arrow { .. } => {
                                let line_b = Bounds {
                                    origin: Point::new(
                                        clipped_ann.origin.x,
                                        clipped_ann.origin.y + clipped_ann.size.height - px(2.0),
                                    ),
                                    size: Size {
                                        width: clipped_ann.size.width,
                                        height: px(2.0),
                                    },
                                };
                                window.paint_quad(fill(line_b, hsla(0.0, 0.9, 0.5, 0.9)));
                            }
                            crate::models::AnnotationStyle::StickyNote { .. } => {
                                let note_b = Bounds {
                                    origin: clipped_ann.origin,
                                    size: Size {
                                        width: px(24.0),
                                        height: px(24.0),
                                    },
                                };
                                window.paint_quad(fill(note_b, hsla(48.0 / 360.0, 1.0, 0.6, 0.95)));
                            }
                            _ => {
                                window.paint_quad(fill(
                                    clipped_ann,
                                    hsla(215.0 / 360.0, 0.9, 0.55, 0.35),
                                ));
                            }
                        }
                    }
                }

                // 2. Paint active selection and word-level text highlighting
                if is_sel_page && let (Some(start), Some(end)) = (sel_start, sel_end) {
                    let sel_win_min_x = start.x.min(end.x);
                    let sel_win_max_x = start.x.max(end.x);
                    let sel_win_min_y = start.y.min(end.y);
                    let sel_win_max_y = start.y.max(end.y);

                    let sel_bounds = Bounds {
                        origin: Point::new(sel_win_min_x, sel_win_min_y),
                        size: Size {
                            width: (start.x - end.x).abs(),
                            height: (start.y - end.y).abs(),
                        },
                    };
                    let clipped = sel_bounds.intersect(&card_bounds);

                    // Convert selection box to page PDF point coordinates
                    let page_min_x = ((sel_win_min_x - card_bounds.origin.x) / px(1.0)) / zoom;
                    let page_max_x = ((sel_win_max_x - card_bounds.origin.x) / px(1.0)) / zoom;
                    let page_min_y = ((sel_win_min_y - card_bounds.origin.y) / px(1.0)) / zoom;
                    let page_max_y = ((sel_win_max_y - card_bounds.origin.y) / px(1.0)) / zoom;

                    let word_fill = hl_color.unwrap_or_else(|| hsla(215.0 / 360.0, 0.9, 0.55, 0.4));
                    let word_border = hl_color
                        .map(|c| c.opacity(0.8))
                        .unwrap_or_else(|| hsla(215.0 / 360.0, 0.9, 0.45, 0.65));

                    let mut matched_any_text = false;

                    if let Some(items) = &text_items {
                        for item in items {
                            let ix2 = item.x + item.width;
                            let iy2 = item.y + item.height;
                            if ix2 >= page_min_x
                                && item.x <= page_max_x
                                && iy2 >= page_min_y
                                && item.y <= page_max_y
                            {
                                matched_any_text = true;
                                let word_quad = Bounds {
                                    origin: Point::new(
                                        card_bounds.origin.x + px(item.x * zoom),
                                        card_bounds.origin.y + px(item.y * zoom),
                                    ),
                                    size: Size {
                                        width: px(item.width * zoom),
                                        height: px(item.height * zoom),
                                    },
                                };
                                let clipped_word = word_quad.intersect(&card_bounds);
                                if clipped_word.size.width > px(1.0)
                                    && clipped_word.size.height > px(1.0)
                                {
                                    window.paint_quad(fill(clipped_word, word_fill));
                                    window.paint_quad(outline(
                                        clipped_word,
                                        word_border,
                                        BorderStyle::Solid,
                                    ));
                                }
                            }
                        }
                    }

                    // While actively dragging, or if no text items matched (e.g. image-only PDF),
                    // paint the drag marquee quad
                    if (is_selecting || !matched_any_text)
                        && clipped.size.width > px(2.0)
                        && clipped.size.height > px(2.0)
                    {
                        let marquee_fill = if matched_any_text {
                            hsla(215.0 / 360.0, 0.85, 0.55, 0.15)
                        } else {
                            word_fill.opacity(0.35)
                        };
                        window.paint_quad(fill(clipped, marquee_fill));
                        window.paint_quad(outline(clipped, word_border, BorderStyle::Solid));
                    }
                }
            },
        )
        .absolute()
        .size_full();

        let card_content = if let Some(render_image) = self.rendered_pages.get(&page_idx) {
            img(render_image.clone())
                .w_full()
                .h_full()
                .into_any_element()
        } else {
            div()
                .size_full()
                .bg(gpui_kit::white())
                .flex()
                .flex_col()
                .justify_between()
                .p_8()
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .justify_between()
                        .text_xs()
                        .text_color(muted_fg)
                        .child(format!("PDFbull Page {}", page_idx + 1))
                        .child(format!("{}/{}", page_idx + 1, total)),
                )
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap_2()
                        .text_color(muted_fg)
                        .child(div().text_sm().child("Rendering page..."))
                        .child(
                            div()
                                .text_xs()
                                .text_color(muted_fg)
                                .child(format!("Page {}", page_idx + 1)),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .justify_center()
                        .text_xs()
                        .text_color(muted_fg)
                        .child(format!(
                            "{} × {} pt",
                            self.page_width.round() as u32,
                            self.page_height.round() as u32
                        )),
                )
                .into_any_element()
        };

        let page_badge = div()
            .absolute()
            .bottom_2()
            .right_2()
            .px_2()
            .py_0p5()
            .rounded_md()
            .bg(gpui_kit::hsla(0.0, 0.0, 0.1, 0.6))
            .text_color(gpui_kit::white())
            .text_xs()
            .font_medium()
            .child(format!("{}/{}", page_idx + 1, total));

        div()
            .id(SharedString::from(format!("canvas-page-{}", page_idx)))
            .w(scaled_w)
            .h(scaled_h)
            .relative()
            .bg(gpui_kit::white())
            .shadow_lg()
            .rounded_sm()
            .border_1()
            .border_color(if is_active { primary } else { border })
            .overflow_hidden()
            .cursor_text()
            .on_mouse_down(MouseButton::Left, down_listener)
            .on_mouse_move(move_listener)
            .on_mouse_up(MouseButton::Left, up_listener)
            .on_click(click_listener)
            .child(card_content)
            .child(selection_overlay)
            .child(page_badge)
            .into_any_element()
    }

    pub fn render<V: 'static>(
        &self,
        cx: &mut Context<V>,
        on_page_click: impl Fn(&mut V, usize, &mut Window, &mut Context<V>) + 'static + Copy,
        on_drag_start: impl Fn(&mut V, usize, Point<Pixels>, &mut Window, &mut Context<V>)
        + 'static
        + Copy,
        on_drag_move: impl Fn(&mut V, usize, Point<Pixels>, &mut Window, &mut Context<V>)
        + 'static
        + Copy,
        on_drag_end: impl Fn(&mut V, usize, &mut Window, &mut Context<V>) + 'static + Copy,
        on_scroll_wheel: impl Fn(&mut V, &ScrollWheelEvent, &mut Window, &mut Context<V>)
        + 'static
        + Copy,
    ) -> AnyElement {
        let muted = cx.theme().muted;
        let cur_page = self.current_page;
        let total = self.total_pages.max(1);

        let scroll_listener = cx.listener(move |v, event: &ScrollWheelEvent, w, cx| {
            on_scroll_wheel(v, event, w, cx);
        });

        match self.layout_mode {
            PageLayoutMode::SinglePage => {
                let card = self.render_page_card(
                    cx,
                    cur_page.min(total - 1),
                    on_page_click,
                    on_drag_start,
                    on_drag_move,
                    on_drag_end,
                );
                div()
                    .id("canvas-single-scroll")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .size_full()
                    .bg(muted)
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll_handle)
                    .vertical_scrollbar(&self.scroll_handle)
                    .on_scroll_wheel(scroll_listener)
                    .items_center()
                    .py_8()
                    .child(card)
                    .into_any_element()
            }
            PageLayoutMode::TwoPageSpread => {
                let (left_idx, right_idx) = if self.standalone_cover {
                    if cur_page == 0 {
                        (0, None)
                    } else {
                        let odd = if cur_page % 2 == 1 {
                            cur_page
                        } else {
                            cur_page - 1
                        };
                        (odd, if odd + 1 < total { Some(odd + 1) } else { None })
                    }
                } else {
                    let even = if cur_page.is_multiple_of(2) {
                        cur_page
                    } else {
                        cur_page - 1
                    };
                    (
                        even,
                        if even + 1 < total {
                            Some(even + 1)
                        } else {
                            None
                        },
                    )
                };

                let left_card = self.render_page_card(
                    cx,
                    left_idx.min(total - 1),
                    on_page_click,
                    on_drag_start,
                    on_drag_move,
                    on_drag_end,
                );
                let right_card = right_idx.map(|r_idx| {
                    self.render_page_card(
                        cx,
                        r_idx.min(total - 1),
                        on_page_click,
                        on_drag_start,
                        on_drag_move,
                        on_drag_end,
                    )
                });

                div()
                    .id("canvas-twopage-scroll")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .size_full()
                    .bg(muted)
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll_handle)
                    .vertical_scrollbar(&self.scroll_handle)
                    .on_scroll_wheel(scroll_listener)
                    .items_center()
                    .py_8()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_center()
                            .gap_6()
                            .child(left_card)
                            .children(right_card),
                    )
                    .into_any_element()
            }
            PageLayoutMode::Continuous => {
                let page_cards: Vec<AnyElement> = (0..total)
                    .map(|idx| {
                        self.render_page_card(
                            cx,
                            idx,
                            on_page_click,
                            on_drag_start,
                            on_drag_move,
                            on_drag_end,
                        )
                    })
                    .collect();

                div()
                    .id("canvas-continuous-scroll")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .size_full()
                    .bg(muted)
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll_handle)
                    .vertical_scrollbar(&self.scroll_handle)
                    .on_scroll_wheel(scroll_listener)
                    .items_center()
                    .py_8()
                    .gap_6()
                    .children(page_cards)
                    .into_any_element()
            }
        }
    }
}
