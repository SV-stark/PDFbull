use gpui_kit::component::ActiveTheme;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::*;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DocumentTab {
    pub id: usize,
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
        }
    }

    pub fn render<V: 'static>(
        &self,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, TabAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> AnyElement {
        let primary = cx.theme().primary;
        let border = cx.theme().border;
        let accent = cx.theme().accent;
        let muted = cx.theme().muted;
        let bg = cx.theme().background;
        let fg = cx.theme().foreground;
        let muted_fg = cx.theme().muted_foreground;
        let active_idx = self.active_tab_index;

        let mut tab_items = Vec::new();
        for (idx, tab) in self.tabs.iter().enumerate() {
            let is_active = idx == active_idx;
            let tab_title = format!("{}{}", tab.title, if tab.is_modified { " *" } else { "" });

            let tab_el = div()
                .id(SharedString::from(format!("tab-item-{}", idx)))
                .flex()
                .flex_row()
                .items_center()
                .h_7()
                .px_2()
                .rounded_md()
                .border_1()
                .border_color(if is_active { primary } else { border })
                .bg(if is_active { accent } else { muted })
                .cursor_pointer()
                .gap_2()
                .on_click(
                    cx.listener(move |v, _, w, cx| on_action(v, TabAction::SelectTab(idx), w, cx)),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(if is_active { fg } else { muted_fg })
                        .child(tab_title),
                )
                .child(
                    Button::new(SharedString::from(format!("close-tab-{}", idx)))
                        .label("×")
                        .ghost()
                        .on_click(cx.listener(move |v, _, w, cx| {
                            on_action(v, TabAction::CloseTab(idx), w, cx)
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
            .items_center()
            .h_9()
            .px_2()
            .bg(bg)
            .border_b_1()
            .border_color(border)
            .gap_1()
            .children(tab_items)
            .child(new_tab_btn)
            .into_any_element()
    }
}
