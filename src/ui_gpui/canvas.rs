use super::ribbon::PageLayoutMode;
use gpui_kit::base::StyledExt;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::gpui::{BorderStyle, fill, outline};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use std::ops::Range;

/// Vertical padding on the canvas scroll container (`.py_8()` = 2rem).
/// Kept in sync with the layout below so scroll maths and layout agree.
const CANVAS_PADDING_Y: f32 = 32.0;
/// Gap between page cards in Continuous mode (`.gap_6()` = 1.5rem).
const CANVAS_PAGE_GAP: f32 = 24.0;
/// Hard cap on the number of page cards built per frame, independent of layout
/// state. ~5 screens' worth at typical page sizes; anything beyond this is
/// cheap to rebuild on the next scroll.
const MAX_CARD_WINDOW: usize = 48;

/// Layout of the virtualized Continuous-mode strip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContinuousStripGeometry {
    /// First page index whose card is built (inclusive).
    pub first: usize,
    /// Last page index whose card is built (exclusive).
    pub last: usize,
    /// Height of the spacer above the first built card.
    pub top_spacer: Pixels,
    /// Height of the spacer below the last built card.
    pub bottom_spacer: Pixels,
    /// Height of one page slot: the card plus the gap beneath it.
    pub slot_height: Pixels,
}

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
    /// `annotations` grouped by page, so painting a page is O(annotations on
    /// that page) instead of a full scan of every annotation.
    pub annotations_by_page: std::collections::HashMap<usize, Vec<crate::models::Annotation>>,
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
            annotations_by_page: std::collections::HashMap::new(),
            search_highlights: std::collections::HashMap::new(),
        }
    }

    /// Drop every piece of per-document state so a new document never inherits
    /// the previous one's pages, annotations, text layer, search hits, cached
    /// page origins or scroll position.
    ///
    /// `rendered_pages` and `rendered_zoom` MUST be cleared together: a zoom
    /// entry without its image (or vice versa) makes `request_render_page`
    /// skip the page forever, leaving it stuck on the "Rendering page..."
    /// placeholder.
    pub fn reset_for_document(&mut self) {
        self.current_page = 0;
        self.is_selecting = false;
        self.selection_page = None;
        self.selection_start = None;
        self.selection_end = None;
        self.selected_text = None;
        self.rendered_pages.clear();
        self.rendered_zoom.clear();
        self.text_cache.clear();
        self.search_highlights.clear();
        self.annotations.clear();
        self.annotations_by_page.clear();
        if let Ok(mut origins) = self.page_origins.write() {
            origins.clear();
        }
        self.scroll_handle.set_offset(Point::default());
    }

    /// Force a re-render of every visible page (zoom, rotation or render
    /// filter changed). Clears the image cache *and* its companion zoom map so
    /// the next `request_render_page` actually issues a command.
    pub fn invalidate_rendered_pages(&mut self) {
        self.rendered_pages.clear();
        self.rendered_zoom.clear();
    }

    /// Page-slot pitch in Continuous mode: one page height plus the gap that
    /// follows it. Every scroll calculation must use this same value.
    pub fn continuous_item_height(&self) -> Pixels {
        px(self.page_height * self.zoom) + px(CANVAS_PAGE_GAP)
    }

    /// Absolute content-space Y of page `page_idx`'s top edge.
    ///
    /// Single source of truth for "where is page N on screen", shared by the
    /// scroll maths and the virtualized layout. If those two ever disagree the
    /// canvas scrolls to the wrong page and the built card window drifts out of
    /// sync with the viewport.
    pub fn page_top_in_content(&self, page_idx: usize) -> Pixels {
        px(CANVAS_PADDING_Y) + self.continuous_item_height() * page_idx as f32
    }

    /// Layout for the virtualized Continuous-mode strip.
    ///
    /// Each built slot is exactly `slot_height` tall and holds a page card
    /// top-aligned, so the gap beneath a card is the slack in its own slot.
    /// That is what makes `page_top_in_content` exact — the scroll container
    /// must therefore carry **no** vertical padding and **no** flex gap, or the
    /// two get double-counted and every card sits a further
    /// `CANVAS_PADDING_Y + CANVAS_PAGE_GAP` below where the scroll maths thinks
    /// it is. That drift is what made continuous scrolling skip pages and left
    /// the last page unreachable at full scroll.
    pub fn continuous_strip_geometry(&self, total: usize) -> ContinuousStripGeometry {
        let slot_height = self.continuous_item_height();
        if total == 0 {
            return ContinuousStripGeometry {
                first: 0,
                last: 0,
                top_spacer: px(0.0),
                bottom_spacer: px(0.0),
                slot_height,
            };
        }
        let window = self.visible_card_window(total);
        ContinuousStripGeometry {
            first: window.start,
            last: window.end,
            top_spacer: self.page_top_in_content(window.start),
            bottom_spacer: px(CANVAS_PADDING_Y) + slot_height * (total - window.end) as f32,
            slot_height,
        }
    }

    pub fn scroll_to_page(&mut self, page_idx: usize) {
        let total = self.total_pages;
        if total == 0 {
            return;
        }
        let target_page = page_idx.min(total.saturating_sub(1));
        self.current_page = target_page;

        match self.layout_mode {
            PageLayoutMode::Continuous => {
                let target_y = -self.page_top_in_content(target_page);
                self.scroll_handle.set_offset(Point::new(px(0.0), target_y));
            }
            PageLayoutMode::SinglePage | PageLayoutMode::TwoPageSpread => {
                self.scroll_handle.set_offset(Point::default());
            }
        }
    }

    pub fn calculate_visible_page_continuous(&self) -> usize {
        let total = self.total_pages;
        if total == 0 {
            return 0;
        }
        let item_h = self.continuous_item_height().max(px(1.0));
        let scrolled_px = (-self.scroll_handle.offset().y).max(px(0.0));
        // Invert `page_top_in_content`: page N occupies
        // `[PADDING + N*item, PADDING + (N+1)*item)`, so the page at the
        // viewport's top edge is `floor((scrolled + PADDING) / item)`.
        let visible_page = ((scrolled_px + px(CANVAS_PADDING_Y)) / item_h).floor() as usize;
        visible_page.min(total.saturating_sub(1))
    }

    /// Half-open range of page indices whose element subtrees should be built
    /// in Continuous mode.
    ///
    /// The window is derived from the live scroll offset (GPUI sets `max_offset`
    /// to `content_size - bounds` on every prepaint, so
    /// `content_size - max_offset` is the viewport height) and is *always*
    /// capped at [`MAX_CARD_WINDOW`] pages. The cap matters: before the first
    /// layout pass `max_offset` is zero, which would make the derived viewport
    /// height equal the whole document and reintroduce the O(total_pages)
    /// build this function exists to avoid.
    ///
    /// The window always contains `current_page`, which is the page the engine
    /// rasterizes.
    pub fn visible_card_window(&self, total: usize) -> Range<usize> {
        if total == 0 {
            return 0..0;
        }
        let anchor = self.current_page.min(total - 1);

        let item_h = self.continuous_item_height();
        if item_h <= px(1.0) {
            return Self::window_around(anchor, total, MAX_CARD_WINDOW);
        }

        // The *real* page count feeds the content height: `max_offset` is
        // `content_size - bounds`, so a clamped page count here would report a
        // viewport a fraction of the actual size and cut the overscan short.
        let content_h = self.page_top_in_content(total) - px(CANVAS_PAGE_GAP);
        let viewport_h = (content_h - self.scroll_handle.max_offset().y).max(px(0.0));
        let scrolled = (-self.scroll_handle.offset().y).max(px(0.0));
        // One viewport of overscan each way keeps cards built ahead of the
        // scroll direction, so fast scrolling never lands on an unbuilt page.
        let overscan = viewport_h + px(96.0);

        let first_px = (scrolled - overscan).max(px(0.0));
        let last_px = scrolled + viewport_h + overscan;

        let derived_start =
            ((first_px / item_h).floor().max(0.0) as usize).min(total.saturating_sub(1));
        let derived_end =
            ((((last_px + item_h) / item_h).ceil() as usize).min(total)).max(derived_start + 1);

        let start = derived_start.min(anchor);
        let end = derived_end.max(anchor + 1);
        if end - start <= MAX_CARD_WINDOW {
            return start..end;
        }
        // Too wide (or, before the first layout, "everything"): fall back to a
        // window centred on the page the engine is actually rendering.
        Self::window_around(anchor, total, MAX_CARD_WINDOW)
    }

    /// At most `count` pages containing `anchor`, biased to put equal amounts
    /// of context on either side. Clamped to `total`.
    fn window_around(anchor: usize, total: usize, count: usize) -> Range<usize> {
        if total <= count {
            return 0..total;
        }
        let room_before = anchor.min(count / 2);
        let room_after = (total - anchor - 1).min(count - 1 - room_before);
        let start = anchor - room_before;
        let end = (anchor + 1 + room_after).min(total);
        start..end
    }

    /// Annotations for `page_idx`, borrowed rather than cloned.
    ///
    /// The per-page `filter(..).cloned()` this replaces was O(total_pages ×
    /// total_annotations) string clones on every frame.
    pub fn annotations_on_page(&self, page_idx: usize) -> &[crate::models::Annotation] {
        self.annotations_by_page
            .get(&page_idx)
            .map_or(&[], Vec::as_slice)
    }

    /// Rebuild `annotations_by_page` from `annotations`. Call after any change
    /// to the annotation list.
    pub fn reindex_annotations(&mut self) {
        self.annotations_by_page.clear();
        for ann in &self.annotations {
            self.annotations_by_page
                .entry(ann.page)
                .or_default()
                .push(ann.clone());
        }
    }

    fn render_page_card<V: 'static>(
        &self,
        cx: &mut Context<V>,
        page_idx: usize,
        on_drag_start: impl Fn(&mut V, usize, Point<Pixels>, &mut Window, &mut Context<V>)
        + 'static
        + Copy,
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
        // on_mouse_move and on_mouse_up are intentionally NOT on the page card.
        // They live on the outer scroll container (added in render()) so that drag
        // events are captured even when the cursor leaves the card bounds or enters
        // the gap between pages. Only on_mouse_down stays per-card so we know which
        // page started the drag.

        let is_sel_page = self.selection_page == Some(page_idx);
        let sel_start = self.selection_start;
        let sel_end = self.selection_end;
        let is_selecting = self.is_selecting;
        // `Arc`-share the per-page text layer and annotations instead of deep
        // cloning them (every `TextItem` and `AnnotationStyle` owns `String`s).
        // The paint closures below only read them.
        let text_items: Option<std::sync::Arc<Vec<crate::models::TextItem>>> = self
            .text_cache
            .get(&page_idx)
            .cloned()
            .map(std::sync::Arc::new);
        let page_annotations: std::sync::Arc<Vec<crate::models::Annotation>> =
            std::sync::Arc::new(self.annotations_on_page(page_idx).to_vec());
        let page_search_matches: std::sync::Arc<Vec<(f32, f32, f32, f32)>> = std::sync::Arc::new(
            self.search_highlights
                .get(&page_idx)
                .cloned()
                .unwrap_or_default(),
        );
        let page_origins = self.page_origins.clone();
        let zoom = self.zoom;
        let hl_color = self.highlight_color;

        let selection_overlay = canvas(
            move |bounds, _, _| bounds,
            move |paint_bounds, card_bounds, window, _| {
                // Record the actual window origin of this page card for
                // coordinate mapping. Skip the write when it hasn't moved —
                // this runs in the paint phase for every visible card, every
                // frame, and a redundant RwLock write per card per frame is
                // pure overhead.
                if let Ok(mut origins) = page_origins.write()
                    && origins.get(&page_idx) != Some(&paint_bounds.origin)
                {
                    origins.insert(page_idx, paint_bounds.origin);
                }

                // 0. Paint search matches on this page
                let search_fill = hsla(45.0 / 360.0, 1.0, 0.5, 0.45);
                for (mx, my, mw, mh) in page_search_matches.iter() {
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
                for ann in page_annotations.iter() {
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

                    // Convert selection box to page PDF point coordinates.
                    // `zoom` is clamped defensively: `f32::clamp` propagates
                    // NaN, and a NaN here would silently blank every selection
                    // and annotation coordinate.
                    let zoom = if zoom.is_finite() && zoom > 0.01 {
                        zoom
                    } else {
                        1.0
                    };
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
                        for item in items.iter() {
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
                        .child(
                            // Static text, deliberately.
                            //
                            // This placeholder used to carry a `Spinner`, which is
                            // `Animation::repeat()` -- an animation that never
                            // ends and so requests a frame forever. In Continuous
                            // mode `visible_card_window` builds up to 48 cards
                            // while only the pages around the cursor are
                            // rasterized, so roughly 40 of them showed this
                            // placeholder. That put ~40 perpetual animations
                            // into the hot path: the window never idled, every
                            // frame rebuilt all 48 page cards, and the app became
                            // unresponsive to input -- including the wheel, so
                            // scrolling appeared dead.
                            div().text_sm().child("Rendering page…"),
                        )
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
            .child(card_content)
            .child(selection_overlay)
            .child(page_badge)
            .into_any_element()
    }

    pub fn render<V: 'static>(
        &self,
        cx: &mut Context<V>,
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

        // Container-level move/up listeners: these fire regardless of which child the
        // cursor is over, so a drag that started on page N and moves into the gap or
        // onto page M still updates selection_end and commits on release.
        // We pass page_idx=0 as a dummy; both closures ignore _page_idx (Bug 2/3 fixes).
        let container_move_listener = cx.listener(move |v, event: &MouseMoveEvent, w, cx| {
            if event.pressed_button == Some(MouseButton::Left) {
                on_drag_move(v, 0, event.position, w, cx);
            }
        });
        let container_up_listener = cx.listener(move |v, _event: &MouseUpEvent, w, cx| {
            on_drag_end(v, 0, w, cx);
        });

        match self.layout_mode {
            PageLayoutMode::SinglePage => {
                let card = self.render_page_card(cx, cur_page.min(total - 1), on_drag_start);
                div()
                    .id("canvas-single-scroll")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .size_full()
                    .bg(muted)
                    .overflow_y_hidden()
                    .track_scroll(&self.scroll_handle)
                    .vertical_scrollbar(&self.scroll_handle)
                    .on_scroll_wheel(scroll_listener)
                    .on_mouse_move(container_move_listener)
                    .on_mouse_up(MouseButton::Left, container_up_listener)
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

                let left_card = self.render_page_card(cx, left_idx.min(total - 1), on_drag_start);
                let right_card = right_idx
                    .map(|r_idx| self.render_page_card(cx, r_idx.min(total - 1), on_drag_start));

                div()
                    .id("canvas-twopage-scroll")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .size_full()
                    .bg(muted)
                    .overflow_y_hidden()
                    .track_scroll(&self.scroll_handle)
                    .vertical_scrollbar(&self.scroll_handle)
                    .on_scroll_wheel(scroll_listener)
                    .on_mouse_move(container_move_listener)
                    .on_mouse_up(MouseButton::Left, container_up_listener)
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
                // Virtualized: only build cards for the pages near the scroll
                // position, with spacers above and below so the total scrollable
                // height (and therefore the scrollbar) is unchanged. Building all
                // `total` cards every frame was O(total_pages) element subtrees
                // per frame and made large documents unusable.
                let strip = self.continuous_strip_geometry(total);

                // Each slot is `slot_height` tall and holds the card top-aligned;
                // the slack beneath it is the inter-page gap. Encoding the gap
                // here (rather than via `.gap_6()` on the container) is what
                // keeps `page_top_in_content` exact — see
                // `continuous_strip_geometry`.
                let slots: Vec<AnyElement> = (strip.first..strip.last)
                    .map(|idx| {
                        div()
                            .flex_none()
                            .h(strip.slot_height)
                            .child(self.render_page_card(cx, idx, on_drag_start))
                            .into_any_element()
                    })
                    .collect();

                div()
                    .id("canvas-continuous-scroll")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .size_full()
                    .bg(muted)
                    // `overflow_y_hidden` (not `overflow_y_scroll`) is
                    // deliberate: `track_scroll` alone already applies the
                    // offset, clamps it and feeds the scrollbar, while
                    // `overflow: Scroll` additionally installs GPUI's own
                    // wheel handler. That handler runs on the same
                    // ScrollHandle, so it applied the wheel delta a second time
                    // on top of the one handled in `handle_canvas_scroll_wheel`
                    // (2x scroll speed), and it also panned the view during
                    // Ctrl+wheel zoom. The app is now the single owner of the
                    // scroll offset.
                    .overflow_y_hidden()
                    .track_scroll(&self.scroll_handle)
                    .vertical_scrollbar(&self.scroll_handle)
                    .on_scroll_wheel(scroll_listener)
                    .on_mouse_move(container_move_listener)
                    .on_mouse_up(MouseButton::Left, container_up_listener)
                    .items_center()
                    // No `.py_8()` and no `.gap_6()`: both are already encoded
                    // in the spacers and slots above. Applying them here as well
                    // shifted every card down by CANVAS_PADDING_Y +
                    // CANVAS_PAGE_GAP relative to the scroll maths.
                    .when(strip.top_spacer > px(0.0), |this| {
                        this.child(div().flex_none().h(strip.top_spacer))
                    })
                    .children(slots)
                    .when(strip.bottom_spacer > px(0.0), |this| {
                        this.child(div().flex_none().h(strip.bottom_spacer))
                    })
                    .into_any_element()
            }
        }
    }
}
