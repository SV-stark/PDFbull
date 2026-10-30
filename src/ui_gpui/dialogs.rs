use gpui_kit::assets::IconName;
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
    /// Edit the document information dictionary (title, author, subject,
    /// keywords, creator, producer).
    Properties,
    About,
    /// Edit the text content of a Text or StickyNote annotation.
    NoteContent,
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
    RotateCurrentPage(i32),
    RotateAllPages(i32),
    DeleteCurrentPage,
    ApplySecurity(String),
    Close,
    /// Field-by-field metadata edit. Each `Option<String>` is `Some` when the
    /// user supplied a value and `None` when the field was left blank — the
    /// engine then leaves that entry untouched rather than blanking it.
    ApplyMetadata {
        title: Option<String>,
        author: Option<String>,
        subject: Option<String>,
        keywords: Option<String>,
        creator: Option<String>,
    },
    /// Replace the text/comment of the annotation currently being edited.
    SetNoteContent(String),
    // Settings & Preferences actions
    SetSettingsTheme(crate::models::AppTheme),
    SetSettingsZoom(u32),
    SetSettingsQuality(crate::pdf_engine::RenderQuality),
    SetSettingsCacheMemory(usize),
    ToggleRestoreSession,
    ToggleRememberLastFile,
    ToggleAutoSave,
    ToggleShowLogsOnStart,
    ResetSettings,
    OpenGitHub,
}

/// Dialogs that present no configurable state must not offer an Apply button
/// that does nothing.
pub fn dialog_has_apply(active: &ActiveDialog) -> bool {
    matches!(
        active,
        ActiveDialog::Watermark
            | ActiveDialog::HeaderFooter
            | ActiveDialog::Security
            | ActiveDialog::Properties
            | ActiveDialog::NoteContent
    )
}

/// Dialogs whose Apply reads through to live editor entities.
pub fn dialog_applies_metadata(active: &ActiveDialog) -> bool {
    matches!(active, ActiveDialog::Properties)
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
    pub password_input_state: Option<Entity<InputState>>,
    pub signatures: Vec<crate::models::SignatureInfo>,
    /// Chosen encryption standard in the Security dialog.
    ///
    /// The Apply button used to send a hard-coded `"AES-256"` regardless of
    /// which option was on screen, so picking AES-128 and pressing Apply
    /// encrypted with AES-256.
    pub security_algorithm: String,
    /// Password editors for the Security dialog.
    pub user_password_input: Option<Entity<InputState>>,
    pub owner_password_input: Option<Entity<InputState>>,
    pub user_password: String,
    pub owner_password: String,
    /// Editors for the Properties dialog's metadata fields, in the order
    /// `[title, author, subject, keywords, creator]`. `None` (or a `None`
    /// element) when built without a window — the dialog then shows the
    /// document's existing values as read-only text.
    pub metadata_inputs: [Option<Entity<InputState>>; 5],
    /// Last-applied metadata, shown as the placeholder when the dialog is
    /// (re)opened. Written by the Apply handler.
    pub metadata_values: [String; 5],
    /// Editor backing the NoteContent dialog.
    pub note_input: Option<Entity<InputState>>,
    /// The annotation id currently being edited in the NoteContent dialog.
    pub note_annotation_id: Option<u64>,
    /// `true` when the edited annotation is a StickyNote (comment) rather
    /// than a Text annotation.
    pub note_is_sticky: bool,
    pub settings: crate::models::AppSettings,
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
        // Placeholders double as the field's own value when the editor is empty.
        const PLACEHOLDERS: [&str; 5] = [
            "Document title",
            "Author name",
            "Subject line",
            "Comma-separated keywords",
            "Creating application",
        ];
        let mut metadata_inputs: [Option<Entity<InputState>>; 5] = [None, None, None, None, None];
        for (slot, ph) in metadata_inputs.iter_mut().zip(PLACEHOLDERS) {
            *slot = Some(owner.new(|cx| InputState::new(window, cx).placeholder(ph)));
        }
        // Note editor.
        let note_input = owner.new(|cx| InputState::new(window, cx).placeholder("Note text"));
        let password_input_state =
            owner.new(|cx| InputState::new(window, cx).placeholder("Password"));
        let user_pw_input =
            owner.new(|cx| InputState::new(window, cx).placeholder("User password (open)"));
        let owner_pw_input =
            owner.new(|cx| InputState::new(window, cx).placeholder("Owner password (permissions)"));
        Self {
            header_input: Some(header_input),
            footer_input: Some(footer_input),
            metadata_inputs,
            note_input: Some(note_input),
            password_input_state: Some(password_input_state),
            user_password_input: Some(user_pw_input),
            owner_password_input: Some(owner_pw_input),
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
            password_input_state: None,
            signatures: Vec::new(),
            security_algorithm: "AES-256".to_string(),
            metadata_inputs: [None, None, None, None, None],
            metadata_values: [
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ],
            note_input: None,
            note_annotation_id: None,
            note_is_sticky: false,
            user_password_input: None,
            owner_password_input: None,
            user_password: String::new(),
            owner_password: String::new(),
            settings: crate::storage::load_settings(),
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
        let primary = cx.theme().primary;
        let accent = cx.theme().accent;

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
                "Customize appearance, rendering performance, and workspace behavior.",
            ),
            ActiveDialog::About => (
                "About PDFbull",
                "High-performance GPU-accelerated PDF Reader & Studio.",
            ),
            ActiveDialog::Properties => (
                "Document Properties",
                "Edit the document information dictionary. Blank fields are left unchanged.",
            ),
            ActiveDialog::NoteContent => ("Annotation Text", "Edit the text of this annotation."),
        };

        let cur_watermark = self.selected_watermark.clone();
        let cur_header = self.header_text.clone();
        let cur_footer = self.footer_text.clone();
        let security_algorithm = self.security_algorithm.clone();
        // Captured up front so the Properties Apply handler can read the
        // editors without reaching into the owning view (this overlay is
        // generic over `V`).
        let meta_inputs = self.metadata_inputs.clone();
        let note_input = self.note_input.clone();
        let password_input_state = self.password_input_state.clone();

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
                let user_pw_input = self
                    .user_password_input
                    .as_ref()
                    .map(|i| Input::new(i).id("dlg-user-pw").w_full());
                let owner_pw_input = self
                    .owner_password_input
                    .as_ref()
                    .map(|i| Input::new(i).id("dlg-owner-pw").w_full());
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
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(muted_fg)
                                    .child("User password (to open)"),
                            )
                            .when_some(user_pw_input, |el, i| el.child(i)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(muted_fg)
                                    .child("Owner password (permissions)"),
                            )
                            .when_some(owner_pw_input, |el, i| el.child(i)),
                    )
                    .child(
                        div().text_xs().text_color(muted_fg).child(
                            "Leave blank to keep the default \"user\" / \"owner\" passwords.",
                        ),
                    )
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
                            Button::new("btn-rot-page-cw")
                                .label("Rotate Page CW")
                                .outline()
                                .on_click(cx.listener(move |v, _, w, cx| {
                                    on_action(v, DialogAction::RotateCurrentPage(90), w, cx)
                                })),
                        )
                        .child(
                            Button::new("btn-rot-page-ccw")
                                .label("Rotate Page CCW")
                                .outline()
                                .on_click(cx.listener(move |v, _, w, cx| {
                                    on_action(v, DialogAction::RotateCurrentPage(-90), w, cx)
                                })),
                        )
                        .child(
                            Button::new("btn-rot-all-cw")
                                .label("Rotate All CW")
                                .outline()
                                .on_click(cx.listener(move |v, _, w, cx| {
                                    on_action(v, DialogAction::RotateAllPages(90), w, cx)
                                })),
                        )
                        .child(
                            Button::new("btn-rot-180")
                                .label("Rotate Page 180°")
                                .outline()
                                .on_click(cx.listener(move |v, _, w, cx| {
                                    on_action(v, DialogAction::RotateCurrentPage(180), w, cx)
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

            ActiveDialog::Properties => {
                // Each row is label + editor. With no editor (tests) the row
                // falls back to showing the last-applied value, so the dialog
                // still renders instead of panicking on an unwrap.
                let labels = ["Title", "Author", "Subject", "Keywords", "Creator"];
                let mut rows = Vec::new();
                for (i, label) in labels.iter().enumerate() {
                    let input = self
                        .metadata_inputs
                        .get(i)
                        .and_then(|slot| slot.clone())
                        .map(|e| Input::new(&e).id(format!("dlg-meta-{i}")).w_full());
                    let current = self.metadata_values.get(i).cloned().unwrap_or_default();
                    let mut row = div().flex().flex_col().gap_1().child(
                        div()
                            .text_xs()
                            .text_color(muted_fg)
                            .child(format!("{label}:")),
                    );
                    row = match input {
                        Some(inp) => row.child(inp),
                        None => row.child(
                            div()
                                .text_sm()
                                .text_color(if current.is_empty() { muted_fg } else { fg })
                                .child(if current.is_empty() {
                                    "(not set)".to_string()
                                } else {
                                    current
                                }),
                        ),
                    };
                    rows.push(row);
                }
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .children(rows)
                    .into_any_element()
            }

            ActiveDialog::NoteContent => {
                let input = self
                    .note_input
                    .as_ref()
                    .map(|i| Input::new(i).id("dlg-note-input").w_full());
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(div().text_xs().text_color(muted_fg).child("Text"))
                            .when_some(input, |el, i| el.child(i)),
                    )
                    .into_any_element()
            }

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
            ActiveDialog::Password => {
                let pw_input = self
                    .password_input_state
                    .as_ref()
                    .map(|i| Input::new(i).id("dlg-pw-input").w_full());
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .text_xs()
                            .text_color(muted_fg)
                            .child("This document is encrypted. Enter its password to open."),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(div().text_xs().text_color(muted_fg).child("Password"))
                            .when_some(pw_input, |el, i| el.child(i)),
                    )
                    .into_any_element()
            }

            ActiveDialog::Settings => {
                let settings = &self.settings;
                let cur_theme = settings.theme.clone();
                let cur_zoom = (settings.default_zoom * 100.0).round() as u32;
                let cur_quality = settings.render_quality;
                let cur_cache = settings.max_cache_memory;
                let restore_session = settings.restore_session;
                let remember_last = settings.remember_last_file;
                let auto_save = settings.auto_save;
                let show_logs = settings.show_logs_on_start;

                let theme_options = [
                    (crate::models::AppTheme::System, "System", IconName::Monitor),
                    (crate::models::AppTheme::Light, "Light", IconName::Sun),
                    (crate::models::AppTheme::Dark, "Dark", IconName::Moon),
                ];
                let mut theme_chips = Vec::new();
                for (th, label, icon) in theme_options {
                    let is_sel = cur_theme == th;
                    let th_clone = th.clone();
                    theme_chips.push(
                        Button::new(format!("setting-theme-{}", label))
                            .label(label)
                            .small()
                            .icon(icon)
                            .when(is_sel, |b| b.primary())
                            .when(!is_sel, |b| b.ghost())
                            .on_click(cx.listener(move |v, _, w, cx| {
                                on_action(
                                    v,
                                    DialogAction::SetSettingsTheme(th_clone.clone()),
                                    w,
                                    cx,
                                )
                            })),
                    );
                }

                let zoom_options = [75, 100, 125, 150, 200];
                let mut zoom_chips = Vec::new();
                for z in zoom_options {
                    let is_sel = cur_zoom == z;
                    zoom_chips.push(
                        Button::new(format!("setting-zoom-{}", z))
                            .label(format!("{}%", z))
                            .small()
                            .when(is_sel, |b| b.primary())
                            .when(!is_sel, |b| b.ghost())
                            .on_click(cx.listener(move |v, _, w, cx| {
                                on_action(v, DialogAction::SetSettingsZoom(z), w, cx)
                            })),
                    );
                }

                let quality_options = [
                    (crate::pdf_engine::RenderQuality::Low, "Low (Fast)"),
                    (
                        crate::pdf_engine::RenderQuality::Medium,
                        "Medium (Balanced)",
                    ),
                    (crate::pdf_engine::RenderQuality::High, "High (Sharp)"),
                ];
                let mut quality_chips = Vec::new();
                for (q, label) in quality_options {
                    let is_sel = cur_quality == q;
                    quality_chips.push(
                        Button::new(format!("setting-quality-{:?}", q))
                            .label(label)
                            .small()
                            .when(is_sel, |b| b.primary())
                            .when(!is_sel, |b| b.ghost())
                            .on_click(cx.listener(move |v, _, w, cx| {
                                on_action(v, DialogAction::SetSettingsQuality(q), w, cx)
                            })),
                    );
                }

                let cache_options = [
                    (256, "256 MB"),
                    (512, "512 MB (Default)"),
                    (1024, "1024 MB (1 GB)"),
                ];
                let mut cache_chips = Vec::new();
                for (mem, label) in cache_options {
                    let is_sel = cur_cache == mem;
                    cache_chips.push(
                        Button::new(format!("setting-cache-{}", mem))
                            .label(label)
                            .small()
                            .when(is_sel, |b| b.primary())
                            .when(!is_sel, |b| b.ghost())
                            .on_click(cx.listener(move |v, _, w, cx| {
                                on_action(v, DialogAction::SetSettingsCacheMemory(mem), w, cx)
                            })),
                    );
                }

                let toggle_row = |id: &'static str,
                                  label: &'static str,
                                  desc: &'static str,
                                  active: bool,
                                  act: DialogAction| {
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .py_1p5()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_0p5()
                                .child(div().text_sm().font_medium().text_color(fg).child(label))
                                .child(div().text_xs().text_color(muted_fg).child(desc)),
                        )
                        .child(
                            Button::new(id)
                                .label(if active { "Enabled" } else { "Disabled" })
                                .small()
                                .when(active, |b| b.primary())
                                .when(!active, |b| b.outline())
                                .on_click(
                                    cx.listener(move |v, _, w, cx| {
                                        on_action(v, act.clone(), w, cx)
                                    }),
                                ),
                        )
                };

                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .max_h(px(400.0))
                    .overflow_y_scrollbar()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1p5()
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(fg)
                                    .child("Theme Mode"),
                            )
                            .child(div().flex().flex_row().gap_2().children(theme_chips)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1p5()
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(fg)
                                    .child("Default Startup Zoom"),
                            )
                            .child(div().flex().flex_row().gap_2().children(zoom_chips)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1p5()
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(fg)
                                    .child("Tile Rendering Quality"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .flex_wrap()
                                    .gap_2()
                                    .children(quality_chips),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1p5()
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(fg)
                                    .child("Raster Cache Memory Limit"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .flex_wrap()
                                    .gap_2()
                                    .children(cache_chips),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .border_t_1()
                            .border_color(border)
                            .pt_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(fg)
                                    .child("Behavior & Workspace"),
                            )
                            .child(toggle_row(
                                "setting-toggle-restore",
                                "Restore Open Tabs",
                                "Re-open previous documents when PDFbull launches",
                                restore_session,
                                DialogAction::ToggleRestoreSession,
                            ))
                            .child(toggle_row(
                                "setting-toggle-remember",
                                "Remember Recent Files",
                                "Keep history of opened files in Welcome dashboard and Jump List",
                                remember_last,
                                DialogAction::ToggleRememberLastFile,
                            ))
                            .child(toggle_row(
                                "setting-toggle-autosave",
                                "Auto-save Annotations",
                                "Save highlights and notes back to the document automatically",
                                auto_save,
                                DialogAction::ToggleAutoSave,
                            ))
                            .child(toggle_row(
                                "setting-toggle-logs",
                                "Diagnostic Console on Startup",
                                "Automatically open bottom debug logs when starting up",
                                show_logs,
                                DialogAction::ToggleShowLogsOnStart,
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .border_t_1()
                            .border_color(border)
                            .pt_2()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(muted_fg)
                                    .child("Settings are saved automatically."),
                            )
                            .child(
                                Button::new("btn-settings-reset")
                                    .label("Reset to Defaults")
                                    .ghost()
                                    .small()
                                    .icon(IconName::RotateCw)
                                    .on_click(cx.listener(move |v, _, w, cx| {
                                        on_action(v, DialogAction::ResetSettings, w, cx)
                                    })),
                            ),
                    )
                    .into_any_element()
            }

            ActiveDialog::About => {
                let spec_row = |name: &'static str, val: &'static str| {
                    div()
                        .flex()
                        .flex_row()
                        .items_baseline()
                        .justify_between()
                        .text_xs()
                        .child(div().font_medium().text_color(muted_fg).child(name))
                        .child(div().font_semibold().text_color(fg).child(val))
                };

                let cap_item = |text: &'static str| {
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_1p5()
                        .text_xs()
                        .text_color(fg)
                        .child(div().text_color(primary).child("•"))
                        .child(div().child(text))
                };

                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .max_h(px(400.0))
                    .overflow_y_scrollbar()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_3()
                            .p_3()
                            .bg(muted)
                            .rounded_lg()
                            .border_1()
                            .border_color(border)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .size_10()
                                    .rounded_lg()
                                    .bg(accent)
                                    .child(Icon::new(IconName::BookOpen).text_color(primary)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_0p5()
                                    .child(
                                        div()
                                            .flex()
                                            .flex_row()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .text_base()
                                                    .font_bold()
                                                    .text_color(fg)
                                                    .child("PDFbull"),
                                            )
                                            .child(
                                                div()
                                                    .px_2()
                                                    .py_0p5()
                                                    .rounded_md()
                                                    .border_1()
                                                    .border_color(border)
                                                    .bg(bg)
                                                    .text_xs()
                                                    .font_semibold()
                                                    .text_color(primary)
                                                    .child(concat!("v", env!("CARGO_PKG_VERSION"))),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(muted_fg)
                                            .child("High-Performance GPU-Accelerated PDF Studio"),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1p5()
                            .p_3()
                            .bg(muted)
                            .rounded_lg()
                            .border_1()
                            .border_color(border)
                            .child(
                                div()
                                    .text_xs()
                                    .font_bold()
                                    .text_color(fg)
                                    .child("Engine & Architecture Specifications"),
                            )
                            .child(spec_row(
                                "Core Engine",
                                "zpdf 0.18.0 (tiny-skia CPU + wgpu GPU)",
                            ))
                            .child(spec_row("Memory Allocator", "mimalloc (High-throughput)"))
                            .child(spec_row(
                                "Memory Safety",
                                "100% Rust, Zero Poppler/MuPDF C++",
                            ))
                            .child(spec_row(
                                "IPC & Messaging",
                                "Single-instance Local Socket IPC",
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .p_3()
                            .bg(muted)
                            .rounded_lg()
                            .border_1()
                            .border_color(border)
                            .child(
                                div()
                                    .text_xs()
                                    .font_bold()
                                    .text_color(fg)
                                    .child("Built-in Features"),
                            )
                            .child(cap_item("Word-level drag text selection & clipboard copy"))
                            .child(cap_item("Multi-document tabs & continuous scrolling"))
                            .child(cap_item(
                                "PDF Annotations: highlight, underline, shapes, notes",
                            ))
                            .child(cap_item("AcroForm inspection, form filling & XFDF export"))
                            .child(cap_item("X.509 Cryptographic digital signature validation"))
                            .child(cap_item("Table detection (CSV) & SVG vector export")),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .pt_1()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_0p5()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_medium()
                                            .text_color(fg)
                                            .child("Author: SV-stark"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(muted_fg)
                                            .child("Released under the MIT License"),
                                    ),
                            )
                            .child(
                                Button::new("btn-about-github")
                                    .label("GitHub Repository")
                                    .outline()
                                    .small()
                                    .icon(IconName::ExternalLink)
                                    .on_click(cx.listener(move |v, _, w, cx| {
                                        on_action(v, DialogAction::OpenGitHub, w, cx)
                                    })),
                            ),
                    )
                    .into_any_element()
            }
        };

        let dialog_width = if matches!(active, ActiveDialog::Signature) {
            px(560.0)
        } else if matches!(active, ActiveDialog::Settings | ActiveDialog::About) {
            px(520.0)
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
                            .label(if dialog_has_apply(&active) {
                                "Cancel"
                            } else {
                                "Close"
                            })
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
                                    ActiveDialog::Properties => {
                                        // Read the editors, mapping an empty
                                        // field to `None` so the engine leaves
                                        // that entry untouched instead of
                                        // blanking it.
                                        let field = |i: usize| -> Option<String> {
                                            let value = meta_inputs
                                                .get(i)?
                                                .clone()?
                                                .read(cx)
                                                .value()
                                                .to_string();
                                            if value.trim().is_empty() {
                                                None
                                            } else {
                                                Some(value)
                                            }
                                        };
                                        on_action(
                                            v,
                                            DialogAction::ApplyMetadata {
                                                title: field(0),
                                                author: field(1),
                                                subject: field(2),
                                                keywords: field(3),
                                                creator: field(4),
                                            },
                                            w,
                                            cx,
                                        );
                                    }
                                    ActiveDialog::NoteContent => {
                                        let text = note_input
                                            .as_ref()
                                            .map(|i| i.read(cx).value().to_string())
                                            .unwrap_or_default();
                                        on_action(v, DialogAction::SetNoteContent(text), w, cx);
                                    }
                                    ActiveDialog::Password => {
                                        let text = password_input_state
                                            .as_ref()
                                            .map(|i| i.read(cx).value().to_string())
                                            .unwrap_or_default();
                                        on_action(v, DialogAction::SetPassword(text), w, cx);
                                    }
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
                .bg(gpui_kit::rgba(0x0000_0080))
                .flex()
                .items_center()
                .justify_center()
                .on_click(cx.listener(move |v, _, w, cx| on_close(v, w, cx)))
                .child(dialog_el)
                .into_any_element(),
        )
    }
}
