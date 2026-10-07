use gpui_kit::base::StyledExt;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Icon;
use gpui_kit::component::Selectable as _;
use gpui_kit::component::Sizable as _;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::prelude::FluentBuilder;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogAction {
    /// Pick a preset in the Watermark dialog. Selects only — it must not close
    /// the dialog, which is what made clicking a chip immediately stamp the
    /// document with the (still default) `CONFIDENTIAL` text.
    SelectWatermark(String),
    SetWatermark(String),
    SetHeaderFooter(String, String),
    SetPassword(String),
    /// Choose an encryption standard without leaving the dialog. Selection only,
    /// like `SelectWatermark` — Apply commits whatever is selected.
    SelectSecurityAlgorithm(String),
    RotatePages(i32),
    DeleteCurrentPage,
    ApplySecurity(String),
    Close,
}

/// Dialogs that present no configurable state must not offer an Apply button
/// that does nothing.
pub fn dialog_has_apply(active: &ActiveDialog) -> bool {
    matches!(
        active,
        ActiveDialog::Watermark | ActiveDialog::HeaderFooter | ActiveDialog::Security
    )
}

/// Algorithms offered by the Security dialog, with the label shown for each.
pub const SECURITY_ALGORITHMS: [(&str, &str); 2] = [
    ("AES-256", "AES-256 (high security)"),
    ("AES-128", "AES-128 (standard)"),
];

pub struct DialogsState {
    pub active: Option<ActiveDialog>,
    pub selected_watermark: String,
    pub header_text: String,
    pub footer_text: String,
    /// Backing editors for the header and footer fields. `None` when the state
    /// was built without a window (tests); the dialog then shows the current
    /// values as static text.
    pub header_input: Option<Entity<InputState>>,
    pub footer_input: Option<Entity<InputState>>,
    pub password_input: String,
    pub signatures: Vec<crate::models::SignatureInfo>,
    /// Chosen encryption standard in the Security dialog.
    ///
    /// The Apply button used to send a hard-coded `"AES-256"` regardless of
    /// which option was on screen, so picking AES-128 and pressing Apply
    /// encrypted with AES-256.
    pub security_algorithm: String,
}

impl Default for DialogsState {
    fn default() -> Self {
        Self::new()
    }
}

impl DialogsState {
    /// Build the dialog state with live editors for the header and footer
    /// fields.
    ///
    /// An `InputState` needs its own `Context`, so this takes the `Context` of
    /// whichever entity owns the dialogs — the same arrangement
    /// [`super::sidebar::SidebarState::new`] uses.
    pub fn new_in<O: 'static>(window: &mut Window, owner: &mut Context<O>) -> Self {
        let header_input =
            owner.new(|cx| InputState::new(window, cx).placeholder("Running header text"));
        let footer_input =
            owner.new(|cx| InputState::new(window, cx).placeholder("Page {page} of {pages}"));
        Self {
            header_input: Some(header_input),
            footer_input: Some(footer_input),
            ..Self::new()
        }
    }

    pub fn new() -> Self {
        Self {
            active: None,
            selected_watermark: "CONFIDENTIAL".to_string(),
            header_text: "Confidential Document".to_string(),
            footer_text: "Page {page} of {pages}".to_string(),
            header_input: None,
            footer_input: None,
            password_input: String::new(),
            signatures: Vec::new(),
            security_algorithm: "AES-256".to_string(),
        }
    }

    pub fn render_overlay<V: 'static>(
        &self,
        cx: &mut Context<V>,
        on_close: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
        on_action: impl Fn(&mut V, DialogAction, &mut Window, &mut Context<V>) + 'static + Copy,
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
                "Apply a prominent diagonal watermark across document pages.",
            ),
            ActiveDialog::HeaderFooter => (
                "Header & Page Numbers",
                "Configure running headers and footer page numbers.",
            ),
            ActiveDialog::Security => (
                "Security & Permissions",
                "Encrypt document with password and restrict printing/copying.",
            ),
            ActiveDialog::Password => (
                "Document Password",
                "This document is encrypted. Enter password to unlock.",
            ),
            ActiveDialog::Signature => (
                "Digital Signatures & Trust Status",
                "Cryptographic byte-range digest & X.509 certificate chain verification.",
            ),
            ActiveDialog::PageOrganizer => {
                ("Visual Page Organizer", "Rotate or remove document pages.")
            }
            ActiveDialog::Settings => (
                "Settings & Preferences",
                "Configure theme, default zoom, and startup behavior.",
            ),
        };

        let cur_watermark = self.selected_watermark.clone();
        let cur_header = self.header_text.clone();
        let cur_footer = self.footer_text.clone();
        let security_algorithm = self.security_algorithm.clone();

        let panel_body: AnyElement = match active {
            ActiveDialog::Watermark => {
                let presets = [
                    "CONFIDENTIAL",
                    "DRAFT",
                    "SAMPLE",
                    "DO NOT COPY",
                    "URGENT",
                    "ORIGINAL",
                ];
                let mut chips = Vec::new();
                for text in presets {
                    let is_sel = cur_watermark == text;
                    // A chip click previously dispatched `SetWatermark`, which
                    // the view handled by *closing* the dialog and applying the
                    // stamp — so picking a preset immediately stamped the
                    // document and `selected_watermark` never changed (Apply
                    // always sent "CONFIDENTIAL"). Chips now only select.
                    chips.push(
                        Button::new(format!("wm-chip-{}", text))
                            .label(text)
                            .ghost()
                            .when(is_sel, |b| b.primary())
                            .on_click(cx.listener(move |v, _, w, cx| {
                                on_action(v, DialogAction::SelectWatermark(text.to_string()), w, cx)
                            })),
                    );
                }

                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .text_xs()
                            .text_color(muted_fg)
                            .child("Select watermark text stamp:"),
                    )
                    .child(div().flex().flex_row().flex_wrap().gap_2().children(chips))
                    .into_any_element()
            }

            ActiveDialog::HeaderFooter => {
                // These two used to be rendered as static text in a bordered
                // box, with no input widget anywhere in the dialog. Apply
                // therefore always sent the constructor defaults, so a person
                // could read "Header Text: Confidential Document", believe they
                // were editing it, and stamp a header they never chose.
                let has_header_editor = self.header_input.is_some();
                let header_input = self
                    .header_input
                    .as_ref()
                    .map(|i| Input::new(i).id("dlg-header-input").w_full());
                let has_footer_editor = self.footer_input.is_some();
                let footer_input = self
                    .footer_input
                    .as_ref()
                    .map(|i| Input::new(i).id("dlg-footer-input").w_full());

                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(div().text_xs().text_color(muted_fg).child("Header text"))
                            .when_some(header_input, |el, i| el.child(i))
                            .when(!has_header_editor, |el| {
                                el.child(
                                    div()
                                        .text_xs()
                                        .text_color(muted_fg)
                                        .child(cur_header.clone()),
                                )
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(div().text_xs().text_color(muted_fg).child("Footer text"))
                            .when_some(footer_input, |el, i| el.child(i))
                            .when(!has_footer_editor, |el| {
                                el.child(
                                    div()
                                        .text_xs()
                                        .text_color(muted_fg)
                                        .child(cur_footer.clone()),
                                )
                            }),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(muted_fg)
                            .child("Use {page} and {pages} in the footer for page numbers."),
                    )
                    .into_any_element()
            }

            ActiveDialog::Security => {
                // Selection only. The buttons used to dispatch `ApplySecurity`
                // *and* close the dialog, so there was no way to change your
                // mind — and the shared Apply button always committed AES-256
                // whatever was on screen.
                let mut buttons = Vec::new();
                for (algo, label) in SECURITY_ALGORITHMS {
                    let is_selected = self.security_algorithm == algo;
                    let algo_owned = algo.to_string();
                    buttons.push(
                        Button::new(format!("sec-{algo}"))
                            .label(label)
                            .small()
                            .selected(is_selected)
                            .on_click(cx.listener(move |v, _, w, cx| {
                                on_action(
                                    v,
                                    DialogAction::SelectSecurityAlgorithm(algo_owned.clone()),
                                    w,
                                    cx,
                                )
                            })),
                    );
                }
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .text_xs()
                            .text_color(muted_fg)
                            .child("Encryption standard"),
                    )
                    .child(div().flex().flex_row().gap_2().children(buttons))
                    .child(div().text_xs().text_color(muted_fg).child(
                        "The document opens with the password \"user\" and the owner \
                                 password \"owner\".",
                    ))
                    .into_any_element()
            }

            ActiveDialog::PageOrganizer => div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .text_xs()
                        .text_color(muted_fg)
                        .child("Page Actions for current document:"),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .gap_2()
                        .child(
                            Button::new("btn-rot-cw")
                                .label("Rotate 90° CW")
                                .outline()
                                .on_click(cx.listener(move |v, _, w, cx| {
                                    on_action(v, DialogAction::RotatePages(90), w, cx)
                                })),
                        )
                        .child(
                            Button::new("btn-rot-ccw")
                                .label("Rotate 90° CCW")
                                .outline()
                                .on_click(cx.listener(move |v, _, w, cx| {
                                    on_action(v, DialogAction::RotatePages(-90), w, cx)
                                })),
                        )
                        .child(
                            Button::new("btn-rot-180")
                                .label("Rotate 180°")
                                .outline()
                                .on_click(cx.listener(move |v, _, w, cx| {
                                    on_action(v, DialogAction::RotatePages(180), w, cx)
                                })),
                        )
                        .child(
                            Button::new("btn-del-page")
                                .label("Delete Current Page")
                                .danger()
                                .on_click(cx.listener(move |v, _, w, cx| {
                                    on_action(v, DialogAction::DeleteCurrentPage, w, cx)
                                })),
                        ),
                )
                .into_any_element(),

            ActiveDialog::Signature => {
                let sigs = self.signatures.clone();
                if sigs.is_empty() {
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .p_6()
                        .gap_2()
                        .bg(muted)
                        .rounded_md()
                        .border_1()
                        .border_color(border)
                        .child(
                            div()
                                .text_sm()
                                .font_bold()
                                .text_color(fg)
                                .child("No Digital Signatures Found"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(muted_fg)
                                .text_center()
                                .child(
                                    "This document does not contain cryptographic signature fields or CMS signatures.",
                                ),
                        )
                        .into_any_element()
                } else {
                    let mut cards = Vec::new();
                    // Signature trust is one of the few places a semantic
                    // status colour is the right answer, but the values come
                    // from the theme's `success` / `warning` / `danger` roles
                    // rather than literal RGBA. The raw literals could not
                    // respond to a custom theme and ignored light-mode
                    // contrast.
                    let success = cx.theme().success;
                    let warning = cx.theme().warning;
                    let danger = cx.theme().danger;
                    let success_fg = cx.theme().success_foreground;
                    let warning_fg = cx.theme().warning_foreground;
                    let danger_fg = cx.theme().danger_foreground;
                    for sig in sigs {
                        let (badge_text, badge_bg, badge_border, badge_text_color, badge_icon) =
                            match sig.badge_kind() {
                                crate::models::SignatureBadgeKind::TrustedRoot => (
                                    "Verified, trusted root CA",
                                    success.opacity(0.15),
                                    success.opacity(0.5),
                                    success_fg,
                                    gpui_kit::component::IconName::CircleCheck,
                                ),
                                crate::models::SignatureBadgeKind::UntrustedRoot => (
                                    "Valid signature, untrusted root",
                                    warning.opacity(0.15),
                                    warning.opacity(0.5),
                                    warning_fg,
                                    gpui_kit::component::IconName::TriangleAlert,
                                ),
                                crate::models::SignatureBadgeKind::Invalid => (
                                    "Invalid signature",
                                    danger.opacity(0.15),
                                    danger.opacity(0.5),
                                    danger_fg,
                                    gpui_kit::component::IconName::CircleX,
                                ),
                            };

                        let signer = sig
                            .signer_name
                            .clone()
                            .unwrap_or_else(|| "Unknown Signer".to_string());
                        let time_str = sig
                            .signing_time
                            .clone()
                            .unwrap_or_else(|| "Date not recorded".to_string());
                        let reason_str = sig
                            .reason
                            .clone()
                            .unwrap_or_else(|| "None specified".to_string());
                        let location_str = sig
                            .location
                            .clone()
                            .unwrap_or_else(|| "Not provided".to_string());

                        let card = div()
                            .flex()
                            .flex_col()
                            .p_3()
                            .gap_2()
                            .bg(muted)
                            .rounded_md()
                            .border_1()
                            .border_color(border)
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .justify_between()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_bold()
                                            .text_color(fg)
                                            .child(format!("{} (Field: {})", signer, sig.field_name)),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_row()
                                            .items_center()
                                            .gap_1()
                                            .px_2()
                                            .py_0p5()
                                            .rounded_full()
                                            .bg(badge_bg)
                                            .border_1()
                                            .border_color(badge_border)
                                            .text_xs()
                                            .font_bold()
                                            .text_color(badge_text_color)
                                            .child(Icon::new(badge_icon))
                                            .child(badge_text),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .text_xs()
                                    .text_color(muted_fg)
                                    .child(format!("• Signing Date: {}", time_str))
                                    .child(format!("• Reason: {}", reason_str))
                                    .child(format!("• Location: {}", location_str))
                                    .child(format!(
                                        "• Cryptographic Integrity: {}",
                                        if sig.crypto_valid && sig.digest_verified {
                                            "Byte-range intact, CMS digest matches document"
                                        } else if !sig.crypto_valid {
                                            "Cryptographic signature check failed"
                                        } else {
                                            "Digest verification failed (document altered after signing)"
                                        }
                                    ))
                                    .when_some(sig.trust_status.clone(), |el, status| {
                                        el.child(format!("• Certificate Chain: {}", status))
                                    }),
                            );

                        cards.push(card);
                    }

                    div()
                        .id("dialog-signatures-scroll")
                        .flex()
                        .flex_col()
                        .gap_3()
                        .max_h(px(320.0))
                        .overflow_y_scrollbar()
                        .children(cards)
                        .into_any_element()
                }
            }

            // The decrypt path does not exist in the engine. This dialog used to
            // fall into a catch-all arm that rendered the literal placeholder
            // text "Document Password Configuration" with no input and no way
            // forward. Say what is actually true instead of showing a form that
            // cannot be filled in.
            ActiveDialog::Password => div()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .p_6()
                .gap_2()
                .bg(muted)
                .rounded_md()
                .border_1()
                .border_color(border)
                .child(
                    div()
                        .text_sm()
                        .font_semibold()
                        .text_color(fg)
                        .child("Opening encrypted documents is not supported yet"),
                )
                .child(div().text_xs().text_color(muted_fg).text_center().child(
                    "PDFbull cannot yet supply the password required to decrypt \
                             a protected PDF.",
                ))
                .into_any_element(),

            ActiveDialog::Settings => div()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .p_6()
                .gap_2()
                .bg(muted)
                .rounded_md()
                .border_1()
                .border_color(border)
                .child(
                    div()
                        .text_sm()
                        .font_semibold()
                        .text_color(fg)
                        .child("Settings are not configurable yet"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(muted_fg)
                        .text_center()
                        .child("Theme, default zoom and startup behaviour are not wired up."),
                )
                .into_any_element(),
        };

        let dialog_width = if matches!(active, ActiveDialog::Signature) {
            px(560.0)
        } else {
            px(480.0)
        };

        let dialog_el = div()
            .id("dialog-card")
            .flex()
            .flex_col()
            .w(dialog_width)
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
            .child(panel_body)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("btn-dialog-cancel")
                            .label("Close")
                            .outline()
                            .on_click(cx.listener(move |v, _, w, cx| on_close(v, w, cx))),
                    )
                    .when(dialog_has_apply(&active), |el| {
                        el.child(
                            Button::new("btn-dialog-apply")
                                .label("Apply")
                                .primary()
                                .on_click(cx.listener(move |v, _, w, cx| match active {
                                    ActiveDialog::Watermark => on_action(
                                        v,
                                        DialogAction::SetWatermark(cur_watermark.clone()),
                                        w,
                                        cx,
                                    ),
                                    ActiveDialog::HeaderFooter => on_action(
                                        v,
                                        DialogAction::SetHeaderFooter(
                                            cur_header.clone(),
                                            cur_footer.clone(),
                                        ),
                                        w,
                                        cx,
                                    ),
                                    // Commit the algorithm that is actually
                                    // selected. This was hard-coded to
                                    // "AES-256", so choosing AES-128 and
                                    // pressing Apply encrypted with AES-256.
                                    ActiveDialog::Security => on_action(
                                        v,
                                        DialogAction::ApplySecurity(security_algorithm.clone()),
                                        w,
                                        cx,
                                    ),
                                    _ => {}
                                })),
                        )
                    }),
            )
            .on_click(cx.listener(|_, _, _, cx| {
                cx.stop_propagation();
            }));

        // Render full screen backdrop with centered modal
        Some(
            div()
                .id("dialog-backdrop")
                .absolute()
                .inset_0()
                .size_full()
                .bg(gpui_kit::rgba(0x00000080))
                .flex()
                .items_center()
                .justify_center()
                .on_click(cx.listener(move |v, _, w, cx| on_close(v, w, cx)))
                .child(dialog_el)
                .into_any_element(),
        )
    }
}
