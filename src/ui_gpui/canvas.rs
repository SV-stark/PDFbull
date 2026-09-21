use super::ribbon::PageLayoutMode;
use gpui_kit::component::ActiveTheme;
use gpui_kit::*;

pub struct DocumentViewport {
    pub zoom: f32,
    pub rotation: u16,
    pub current_page: usize,
    pub total_pages: usize,
    pub page_width: f32,
    pub page_height: f32,
    pub layout_mode: PageLayoutMode,
    pub standalone_cover: bool,
    pub selection_start: Option<Point<Pixels>>,
    pub selection_end: Option<Point<Pixels>>,
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
            selection_start: None,
            selection_end: None,
        }
    }

    pub fn render<V: 'static>(
        &self,
        cx: &mut Context<V>,
        on_page_click: impl Fn(&mut V, usize, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> AnyElement {
        let primary = cx.theme().primary;
        let border = cx.theme().border;
        let muted = cx.theme().muted;
        let muted_fg = cx.theme().muted_foreground;
        let scaled_w = px(self.page_width * self.zoom);
        let scaled_h = px(self.page_height * self.zoom);
        let cur_page = self.current_page;
        let total = self.total_pages;

        let mut page_cards = Vec::new();
        for page_idx in 0..total {
            let is_active = page_idx == cur_page;
            let click_listener = cx.listener(move |v, _, w, cx| on_page_click(v, page_idx, w, cx));

            page_cards.push(
                div()
                    .id(SharedString::from(format!("canvas-page-{}", page_idx)))
                    .w(scaled_w)
                    .h(scaled_h)
                    .bg(gpui_kit::white())
                    .shadow_lg()
                    .rounded_sm()
                    .border_1()
                    .border_color(if is_active { primary } else { border })
                    .cursor_text()
                    .flex()
                    .flex_col()
                    .justify_between()
                    .p_8()
                    .on_click(click_listener)
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
                            .items_center()
                            .justify_center()
                            .text_color(muted_fg)
                            .child(format!("Document Viewport — Page {}", page_idx + 1)),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_center()
                            .text_xs()
                            .text_color(muted_fg)
                            .child(format!("{} × {} pt", 595, 842)),
                    ),
            );
        }

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(muted)
            .overflow_hidden()
            .items_center()
            .py_8()
            .gap_6()
            .children(page_cards)
            .into_any_element()
    }
}
