use crate::app::{INTER_BOLD, INTER_REGULAR, PdfBullApp};
use crate::message::Message;
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Border, Color, Element, Length, Padding};

pub struct ProfileInfo {
    pub idx: usize,
    pub name: &'static str,
    pub tag: &'static str,
    pub desc: &'static str,
}

pub const PROFILES_A: &[ProfileInfo] = &[
    ProfileInfo {
        idx: 0,
        name: "PDF/A-1b",
        tag: "ISO 19005-1",
        desc: "Archival basic visual preservation (device-independent color, embedded fonts)",
    },
    ProfileInfo {
        idx: 1,
        name: "PDF/A-2b",
        tag: "ISO 19005-2",
        desc: "Archival standard supporting JPEG2000, transparency, and layers",
    },
    ProfileInfo {
        idx: 2,
        name: "PDF/A-3b",
        tag: "ISO 19005-3",
        desc: "Archival standard supporting arbitrary embedded file attachments",
    },
];

pub const PROFILES_X: &[ProfileInfo] = &[
    ProfileInfo {
        idx: 3,
        name: "PDF/X-1a",
        tag: "ISO 15930-1",
        desc: "Prepress blind exchange (restricted to CMYK and spot colors)",
    },
    ProfileInfo {
        idx: 4,
        name: "PDF/X-3",
        tag: "ISO 15930-3",
        desc: "Prepress standard supporting color-managed ICC workflows",
    },
    ProfileInfo {
        idx: 5,
        name: "PDF/X-4",
        tag: "ISO 15930-7",
        desc: "Prepress standard supporting live transparency and layer OCGs",
    },
    ProfileInfo {
        idx: 6,
        name: "PDF/X-6",
        tag: "ISO 15930-9",
        desc: "Modern prepress standard based on PDF 2.0 with page-level output intents",
    },
];

pub const PROFILES_UA: &[ProfileInfo] = &[
    ProfileInfo {
        idx: 7,
        name: "PDF/UA-1",
        tag: "ISO 14289-1",
        desc: "Universal accessibility (structure hierarchy, alternative text, reading order)",
    },
    ProfileInfo {
        idx: 8,
        name: "PDF/UA-2",
        tag: "ISO 14289-2",
        desc: "Modern accessibility standard built on PDF 2.0 structure elements",
    },
];

fn profile_button(info: &ProfileInfo, selected: bool) -> Element<'static, Message> {
    let idx = info.idx;
    let name_color = if selected {
        Color::from_rgb8(100, 200, 255)
    } else {
        Color::from_rgb8(230, 230, 230)
    };
    let tag_color = if selected {
        Color::from_rgb8(140, 210, 255)
    } else {
        Color::from_rgb8(140, 140, 150)
    };

    button(
        column![
            row![
                text(info.name).size(14).font(INTER_BOLD).style(move |_| {
                    iced::widget::text::Style {
                        color: Some(name_color),
                    }
                }),
                Space::new().width(Length::Fill),
                text(info.tag).size(11).font(INTER_REGULAR).style(move |_| {
                    iced::widget::text::Style {
                        color: Some(tag_color),
                    }
                }),
            ]
            .align_y(Alignment::Center),
            text(info.desc).size(12).font(INTER_REGULAR).style(|_| {
                iced::widget::text::Style {
                    color: Some(Color::from_rgb8(160, 160, 170)),
                }
            }),
        ]
        .spacing(4),
    )
    .on_press(Message::SetConformanceProfile(idx))
    .padding(Padding::from([10, 14]))
    .width(Length::Fill)
    .style(move |_, status| {
        let is_hovered = matches!(status, iced::widget::button::Status::Hovered);
        let bg = if selected {
            Color::from_rgba(0.18, 0.35, 0.60, 0.45)
        } else if is_hovered {
            Color::from_rgba(1.0, 1.0, 1.0, 0.08)
        } else {
            Color::from_rgba(1.0, 1.0, 1.0, 0.03)
        };
        let border_col = if selected {
            Color::from_rgb8(70, 130, 220)
        } else if is_hovered {
            Color::from_rgba(1.0, 1.0, 1.0, 0.2)
        } else {
            Color::from_rgba(1.0, 1.0, 1.0, 0.08)
        };
        iced::widget::button::Style {
            background: Some(bg.into()),
            text_color: Color::WHITE,
            border: Border {
                radius: 8.0.into(),
                width: 1.0,
                color: border_col,
            },
            ..Default::default()
        }
    })
    .into()
}

pub fn conformance_validator_view(app: &PdfBullApp) -> Element<'_, Message> {
    let has_doc = app.current_tab().is_some();

    // ── Header ───────────────────────────────────────────────────────────────
    let header_row = row![
        column![
            text("\u{1f6e1}  PDF Conformance Validator")
                .size(20)
                .font(INTER_BOLD)
                .style(|_| iced::widget::text::Style {
                    color: Some(Color::WHITE),
                }),
            text("Verify compliance against PDF/A, PDF/X, and PDF/UA international standards")
                .size(13)
                .font(INTER_REGULAR)
                .style(|_| iced::widget::text::Style {
                    color: Some(Color::from_rgb8(160, 160, 170)),
                }),
        ]
        .spacing(4),
        Space::new().width(Length::Fill),
        button(text("\u{2715} Close").size(13).font(INTER_REGULAR))
            .on_press(Message::ToggleConformanceValidator(false))
            .padding([6, 12])
            .style(iced::widget::button::secondary),
    ]
    .align_y(Alignment::Center);

    // ── Profile Cards ─────────────────────────────────────────────────────────
    let a_buttons: Vec<Element<'_, Message>> = PROFILES_A
        .iter()
        .map(|p| profile_button(p, app.conformance_profile_idx == p.idx))
        .collect();

    let x_buttons: Vec<Element<'_, Message>> = PROFILES_X
        .iter()
        .map(|p| profile_button(p, app.conformance_profile_idx == p.idx))
        .collect();

    let ua_buttons: Vec<Element<'_, Message>> = PROFILES_UA
        .iter()
        .map(|p| profile_button(p, app.conformance_profile_idx == p.idx))
        .collect();

    let section_a = column![
        text("PDF/A  (Archiving & Long-Term Preservation)")
            .size(13)
            .font(INTER_BOLD)
            .style(|_| iced::widget::text::Style {
                color: Some(Color::from_rgb8(100, 190, 255)),
            }),
        column(a_buttons).spacing(6),
    ]
    .spacing(8);

    let section_x = column![
        text("PDF/X  (Prepress & Graphic Exchange)")
            .size(13)
            .font(INTER_BOLD)
            .style(|_| iced::widget::text::Style {
                color: Some(Color::from_rgb8(240, 160, 80)),
            }),
        column(x_buttons).spacing(6),
    ]
    .spacing(8);

    let section_ua = column![
        text("PDF/UA  (Universal Accessibility)")
            .size(13)
            .font(INTER_BOLD)
            .style(|_| iced::widget::text::Style {
                color: Some(Color::from_rgb8(120, 220, 140)),
            }),
        column(ua_buttons).spacing(6),
    ]
    .spacing(8);

    let profile_selector = column![section_a, section_x, section_ua].spacing(16);

    // ── Action Bar ────────────────────────────────────────────────────────────
    let validate_btn: Element<'_, Message> = if app.conformance_pending {
        button(text("Validating document...").size(14).font(INTER_BOLD))
            .padding([10, 24])
            .style(iced::widget::button::secondary)
            .into()
    } else if has_doc {
        button(
            text("\u{25b6}  Validate Document")
                .size(14)
                .font(INTER_BOLD),
        )
        .on_press(Message::RunConformanceValidation)
        .padding([10, 24])
        .style(iced::widget::button::primary)
        .into()
    } else {
        button(text("No Document Open").size(14).font(INTER_BOLD))
            .padding([10, 24])
            .style(iced::widget::button::secondary)
            .into()
    };

    let action_bar =
        row![Space::new().width(Length::Fill), validate_btn,].align_y(Alignment::Center);

    // ── Results Panel ─────────────────────────────────────────────────────────
    let results_panel: Element<'_, Message> = if let Some(report) = &app.conformance_report {
        let (badge_text, badge_bg, badge_border) = if report.conforms {
            (
                "\u{2713}  CONFORMS",
                Color::from_rgba(0.12, 0.45, 0.20, 0.35),
                Color::from_rgb8(60, 180, 90),
            )
        } else {
            (
                "\u{2717}  DOES NOT CONFORM",
                Color::from_rgba(0.55, 0.15, 0.15, 0.35),
                Color::from_rgb8(220, 70, 70),
            )
        };

        let banner = container(
            row![
                text(badge_text).size(14).font(INTER_BOLD).style(move |_| {
                    iced::widget::text::Style {
                        color: Some(badge_border),
                    }
                }),
                Space::new().width(12),
                text(format!(
                    "Profile: {} \u{2022} {} violation(s)",
                    report.profile,
                    report.violations.len()
                ))
                .size(13)
                .font(INTER_REGULAR)
                .style(|_| iced::widget::text::Style {
                    color: Some(Color::from_rgb8(220, 220, 225)),
                }),
            ]
            .align_y(Alignment::Center),
        )
        .padding(Padding::from([12, 16]))
        .width(Length::Fill)
        .style(move |_| iced::widget::container::Style {
            background: Some(badge_bg.into()),
            border: Border {
                radius: 8.0.into(),
                width: 1.0,
                color: badge_border,
            },
            ..Default::default()
        });

        let claimed_view: Element<'_, Message> = if !report.claimed.is_empty() {
            let pills = report.claimed.iter().map(|(k, v)| {
                container(
                    text(format!("{k}: {v}"))
                        .size(12)
                        .font(INTER_BOLD)
                        .style(|_| iced::widget::text::Style {
                            color: Some(Color::from_rgb8(180, 210, 255)),
                        }),
                )
                .padding([4, 8])
                .style(|_| iced::widget::container::Style {
                    background: Some(Color::from_rgba(0.2, 0.4, 0.8, 0.25).into()),
                    border: Border {
                        radius: 4.0.into(),
                        width: 1.0,
                        color: Color::from_rgb8(70, 120, 200),
                    },
                    ..Default::default()
                })
                .into()
            });

            row![
                text("Claimed Metadata: ")
                    .size(12)
                    .font(INTER_REGULAR)
                    .style(|_| iced::widget::text::Style {
                        color: Some(Color::from_rgb8(150, 150, 160)),
                    }),
                row(pills).spacing(8),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
        } else {
            Space::new().height(0).into()
        };

        let violations_view: Element<'_, Message> = if report.violations.is_empty() {
            container(
                text(if report.conforms {
                    "No rule violations found. The document conforms to all checked criteria."
                } else {
                    "Document failed conformance validation."
                })
                .size(13)
                .font(INTER_REGULAR)
                .style(|_| iced::widget::text::Style {
                    color: Some(Color::from_rgb8(150, 210, 150)),
                }),
            )
            .padding(12)
            .into()
        } else {
            let list: Vec<Element<'_, Message>> = report
                .violations
                .iter()
                .map(|v| {
                    container(
                        column![
                            row![text(&v.rule).size(12).font(INTER_BOLD).style(|_| {
                                iced::widget::text::Style {
                                    color: Some(Color::from_rgb8(255, 130, 130)),
                                }
                            }),],
                            text(&v.message).size(13).font(INTER_REGULAR).style(|_| {
                                iced::widget::text::Style {
                                    color: Some(Color::from_rgb8(215, 215, 220)),
                                }
                            }),
                        ]
                        .spacing(4),
                    )
                    .padding(Padding::from([10, 12]))
                    .width(Length::Fill)
                    .style(|_| iced::widget::container::Style {
                        background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.03).into()),
                        border: Border {
                            radius: 6.0.into(),
                            width: 1.0,
                            color: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                        },
                        ..Default::default()
                    })
                    .into()
                })
                .collect();

            scrollable(column(list).spacing(8))
                .height(Length::Fixed(220.0))
                .into()
        };

        column![banner, claimed_view, violations_view]
            .spacing(12)
            .into()
    } else {
        container(
            text("Select a standard profile above and click 'Validate Document' to inspect.")
                .size(13)
                .font(INTER_REGULAR)
                .style(|_| iced::widget::text::Style {
                    color: Some(Color::from_rgb8(140, 140, 150)),
                }),
        )
        .padding(16)
        .center_x(Length::Fill)
        .style(|_| iced::widget::container::Style {
            background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.02).into()),
            border: Border {
                radius: 8.0.into(),
                width: 1.0,
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.06),
            },
            ..Default::default()
        })
        .into()
    };

    // ── Dialog Container ──────────────────────────────────────────────────────
    let content = column![header_row, profile_selector, action_bar, results_panel,].spacing(18);

    container(scrollable(
        container(content)
            .width(Length::Fixed(720.0))
            .padding(28)
            .style(|_| iced::widget::container::Style {
                background: Some(Color::from_rgb8(36, 38, 42).into()),
                border: Border {
                    radius: 12.0.into(),
                    width: 1.0,
                    color: Color::from_rgb8(60, 62, 68),
                },
                ..Default::default()
            }),
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .style(|_| iced::widget::container::Style {
        background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.72).into()),
        ..Default::default()
    })
    .into()
}
