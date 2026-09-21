use gpui_kit::component::ActiveTheme;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveDialog {
    Watermark,
    HeaderFooter,
    Security,
    Password,
    Signature,
    PageOrganizer,
    Settings,
}

pub struct DialogsState {
    pub active: Option<ActiveDialog>,
}

impl Default for DialogsState {
    fn default() -> Self {
        Self::new()
    }
}

impl DialogsState {
    pub fn new() -> Self {
        Self { active: None }
    }

    pub fn render_overlay<V: 'static>(
        &self,
        cx: &mut Context<V>,
        on_close: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
        on_apply: impl Fn(&mut V, ActiveDialog, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Option<AnyElement> {
        let active = self.active.clone()?;
        let bg = cx.theme().background;
        let border = cx.theme().border;
        let fg = cx.theme().foreground;
        let muted = cx.theme().muted;
        let muted_fg = cx.theme().muted_foreground;

        let (title, description): (&str, &str) = match active {
            ActiveDialog::Watermark => (
                "Add Watermark",
                "Apply text or image watermark across document pages.",
            ),
            ActiveDialog::HeaderFooter => (
                "Header & Page Numbers",
                "Configure running headers, footers, and page numbers.",
            ),
            ActiveDialog::Security => (
                "Security & Permissions",
                "Set user/owner passwords and restrict printing/copying.",
            ),
            ActiveDialog::Password => (
                "Document Password",
                "This document is encrypted. Please enter the password.",
            ),
            ActiveDialog::Signature => (
                "Digital Signature",
                "Place a cryptographic digital signature on the document.",
            ),
            ActiveDialog::PageOrganizer => (
                "Visual Page Organizer",
                "Reorder, rotate, extract, or delete pages.",
            ),
            ActiveDialog::Settings => (
                "Settings & Preferences",
                "Configure theme, default zoom, and startup behavior.",
            ),
        };

        let dialog_el = div()
            .flex()
            .flex_col()
            .w(px(480.0))
            .bg(bg)
            .border_1()
            .border_color(border)
            .rounded_lg()
            .shadow_xl()
            .p_6()
            .gap_4()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_lg().text_color(fg).child(title))
                    .child(div().text_sm().text_color(muted_fg).child(description)),
            )
            .child(
                div()
                    .h_32()
                    .bg(muted)
                    .rounded_md()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_sm()
                    .text_color(muted_fg)
                    .child(format!("{} Configuration Panel", title)),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("btn-dialog-cancel")
                            .label("Cancel")
                            .outline()
                            .on_click(cx.listener(move |v, _, w, cx| on_close(v, w, cx))),
                    )
                    .child(
                        Button::new("btn-dialog-apply")
                            .label("Apply")
                            .primary()
                            .on_click(
                                cx.listener(move |v, _, w, cx| on_apply(v, active.clone(), w, cx)),
                            ),
                    ),
            );

        // Render full screen backdrop with centered modal
        Some(
            div()
                .absolute()
                .size_full()
                .bg(gpui_kit::rgba(0x00000080))
                .flex()
                .items_center()
                .justify_center()
                .child(dialog_el)
                .into_any_element(),
        )
    }
}
