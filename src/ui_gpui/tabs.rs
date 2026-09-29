use gpui_kit::base::StyledExt;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DocumentTab {
    /// Stable, monotonically increasing identity for this tab. Used for element
    /// ids so a tab keeps its hover/focus state when the list is reordered.
    pub id: u64,
    pub doc_id: Option<crate::models::DocumentId>,
    pub title: String,
    pub path: Option<PathBuf>,
    pub is_modified: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabAction {
    SelectTab(usize),
    CloseTab(usize),
    NewTab,
    CloseOthers(usize),
    CloseToRight(usize),
}

pub struct TabsState {
    pub tabs: Vec<DocumentTab>,
    pub active_tab_index: usize,
    /// Next stable tab id to hand out (never reused, so element ids stay unique).
    next_id: u64,
}

impl Default for TabsState {
    fn default() -> Self {
        Self::new()
    }
}

impl TabsState {
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active_tab_index: 0,
            next_id: 1,
        }
    }

    /// Append a tab and return its stable id.
    pub fn push(&mut self, tab: DocumentTab) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.tabs.push(DocumentTab { id, ..tab });
        id
    }

    pub fn render<V: 'static>(
        &self,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, TabAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> AnyElement {
        let primary = cx.theme().primary;
        let border = cx.theme().border;
        let bg = cx.theme().background;
        let fg = cx.theme().foreground;
        let muted = cx.theme().muted;
        let muted_fg = cx.theme().muted_foreground;
        let active_idx = self.active_tab_index;

        let mut tab_items = Vec::new();
        for (idx, tab) in self.tabs.iter().enumerate() {
            let is_active = idx == active_idx;
            // Element ids are derived from the tab's stable id, not its
            // position. Position-based ids shift whenever a tab is closed or
            // inserted, which handed one tab's retained focus/hover state to a
            // different document.
            let tab_id = tab.id;

            let tab_el = div()
                .id(SharedString::from(format!("tab-item-{}", tab_id)))
                .flex()
                .flex_row()
                .items_center()
                .h(px(32.0))
                .px_2p5()
                .rounded_t_md()
                .border_1()
                .border_color(if is_active { primary } else { border })
                .bg(if is_active {
                    bg
                } else {
                    gpui_kit::transparent_black()
                })
                .hover(move |s| if !is_active { s.bg(muted) } else { s })
                .cursor_pointer()
                .gap_2()
                .on_click(cx.listener(move |v, _, w, cx| {
                    on_action(v, TabAction::SelectTab(idx), w, cx);
                }))
                .child(div().text_xs().child("📄"))
                .child(
                    div()
                        .text_xs()
                        .font_medium()
                        .text_color(if is_active { fg } else { muted_fg })
                        .child(tab.title.clone()),
                )
                .when(tab.is_modified, |this| {
                    this.child(div().size_2().rounded_full().bg(primary))
                })
                .child(
                    Button::new(SharedString::from(format!("close-tab-{}", tab_id)))
                        .label("×")
                        .ghost()
                        .on_click(cx.listener(move |v, _, w, cx| {
                            // The close button is nested inside the tab's own
                            // clickable div. gpui-kit's Button only stops click
                            // propagation when disabled or loading, and GPUI
                            // dispatches bubble-phase listeners in reverse paint
                            // order (child first), so without this the parent
                            // also received SelectTab(idx) — which ran *after*
                            // CloseTab(idx) and reset `active_tab_index` to the
                            // index of the tab that had just been removed.
                            cx.stop_propagation();
                            on_action(v, TabAction::CloseTab(idx), w, cx);
                        })),
                );
            tab_items.push(tab_el);
        }

        let new_tab_btn = Button::new("btn-new-tab")
            .label("+")
            .ghost()
            .on_click(cx.listener(move |v, _, w, cx| on_action(v, TabAction::NewTab, w, cx)));

        div()
            .flex()
            .flex_row()
            .items_end()
            .h(px(36.0))
            .px_2()
            .pt_1()
            .bg(muted)
            .border_b_1()
            .border_color(border)
            .gap_1()
            .children(tab_items)
            .child(div().pb_0p5().child(new_tab_btn))
            .into_any_element()
    }
}
