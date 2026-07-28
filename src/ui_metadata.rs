use crate::app::{INTER_BOLD, INTER_REGULAR, PdfBullApp};
use crate::message::Message;
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Color, Element, Length};

pub fn metadata_view(app: &PdfBullApp) -> Element<'_, Message> {
    let Some(tab) = app.current_tab() else {
        return container(text("No document open")).into();
    };

    let meta = &tab.metadata;

    let header_row = row![
        text("Document Information").size(24).font(INTER_BOLD),
        Space::new().width(Length::Fill),
        button(text("Close").size(14).font(INTER_REGULAR))
            .on_press(Message::ToggleMetadata)
            .padding([6, 12])
            .style(iced::widget::button::text),
    ]
    .align_y(Alignment::Center);

    let fields: Vec<(String, String)> = vec![
        (
            "Title".to_string(),
            meta.title.as_deref().unwrap_or("N/A").to_string(),
        ),
        (
            "Author".to_string(),
            meta.author.as_deref().unwrap_or("N/A").to_string(),
        ),
        (
            "Subject".to_string(),
            meta.subject.as_deref().unwrap_or("N/A").to_string(),
        ),
        (
            "Keywords".to_string(),
            meta.keywords.as_deref().unwrap_or("N/A").to_string(),
        ),
        (
            "Creator".to_string(),
            meta.creator.as_deref().unwrap_or("N/A").to_string(),
        ),
        (
            "Producer".to_string(),
            meta.producer.as_deref().unwrap_or("N/A").to_string(),
        ),
        (
            "Creation Date".to_string(),
            meta.creation_date.as_deref().unwrap_or("N/A").to_string(),
        ),
        (
            "Modification Date".to_string(),
            meta.modification_date
                .as_deref()
                .unwrap_or("N/A")
                .to_string(),
        ),
        (
            "File Path".to_string(),
            tab.path.to_string_lossy().into_owned(),
        ),
        ("Page Count".to_string(), tab.total_pages.to_string()),
    ];

    let meta_table = iced::widget::table(
        [
            iced::widget::table::column(
                "Property",
                |row: (String, String)| -> Element<'_, Message> {
                    text(row.0)
                        .size(14)
                        .font(INTER_BOLD)
                        .style(|_| iced::widget::text::Style {
                            color: Some(Color::from_rgb(0.5, 0.5, 0.5)),
                        })
                        .into()
                },
            ),
            iced::widget::table::column("Value", |row: (String, String)| -> Element<'_, Message> {
                text(row.1).size(16).font(INTER_REGULAR).into()
            }),
        ],
        fields,
    );

    // ── Feature 5: CMYK / ICC Color Profile section ───────────────────────────
    let color_section: Element<'_, Message> = if let Some(profile) = &tab.color_profile {
        let color_fields: Vec<(String, String)> = vec![
            (
                "Output Intent".to_string(),
                profile
                    .output_intent_name
                    .as_deref()
                    .unwrap_or("Unknown")
                    .to_string(),
            ),
            (
                "Output Condition".to_string(),
                profile
                    .output_condition
                    .as_deref()
                    .unwrap_or("N/A")
                    .to_string(),
            ),
            (
                "CMYK ICC Profile".to_string(),
                if profile.has_cmyk_profile {
                    "Present \u{2713}".to_string()
                } else {
                    "Not found".to_string()
                },
            ),
            (
                "ICC Profile Embedded".to_string(),
                if profile.has_icc_profile {
                    "Yes \u{2713}".to_string()
                } else {
                    "No".to_string()
                },
            ),
        ];
        let color_table = iced::widget::table(
            [
                iced::widget::table::column(
                    "Property",
                    |row: (String, String)| -> Element<'_, Message> {
                        text(row.0)
                            .size(14)
                            .font(INTER_BOLD)
                            .style(|_| iced::widget::text::Style {
                                color: Some(Color::from_rgb(0.5, 0.5, 0.5)),
                            })
                            .into()
                    },
                ),
                iced::widget::table::column(
                    "Value",
                    |row: (String, String)| -> Element<'_, Message> {
                        text(row.1).size(16).font(INTER_REGULAR).into()
                    },
                ),
            ],
            color_fields,
        );
        column![
            text("\u{1f5a8}\u{fe0f}  Print Color Profiles")
                .size(18)
                .font(INTER_BOLD)
                .style(|_| iced::widget::text::Style {
                    color: Some(Color::from_rgb8(120, 180, 255)),
                }),
            color_table,
        ]
        .spacing(10)
        .into()
    } else {
        column![
            text("\u{1f5a8}\u{fe0f}  Print Color Profiles")
                .size(18)
                .font(INTER_BOLD)
                .style(|_| iced::widget::text::Style {
                    color: Some(Color::from_rgb(0.4, 0.4, 0.4)),
                }),
            text("No output intent or ICC profile embedded in this document.")
                .size(14)
                .font(INTER_REGULAR)
                .style(|_| iced::widget::text::Style {
                    color: Some(Color::from_rgb(0.5, 0.5, 0.5)),
                }),
        ]
        .spacing(6)
        .into()
    };

    // ── Feature 4: Geospatial / GIS section ──────────────────────────────────
    let geo_section: Element<'_, Message> = if tab.geo_annotations.is_empty() {
        column![
            text("\u{1f4cd}  Geospatial Data")
                .size(18)
                .font(INTER_BOLD)
                .style(|_| iced::widget::text::Style {
                    color: Some(Color::from_rgb(0.4, 0.4, 0.4)),
                }),
            text("No geospatial /Measure annotations found (not a GeoPDF).")
                .size(14)
                .font(INTER_REGULAR)
                .style(|_| iced::widget::text::Style {
                    color: Some(Color::from_rgb(0.5, 0.5, 0.5)),
                }),
        ]
        .spacing(6)
        .into()
    } else {
        let mut geo_fields: Vec<(String, String)> = Vec::new();
        for (i, geo) in tab.geo_annotations.iter().enumerate() {
            let prefix = if tab.geo_annotations.len() > 1 {
                format!("Annotation {} \u{2014} ", i + 1)
            } else {
                String::new()
            };
            geo_fields.push((format!("{prefix}Page"), (geo.page + 1).to_string()));
            if let Some(cs) = &geo.coordinate_system {
                geo_fields.push((format!("{prefix}Coordinate System"), cs.clone()));
            }
            if let Some(proj) = &geo.projection_name {
                geo_fields.push((format!("{prefix}Projection"), proj.clone()));
            }
            if let Some(scale) = geo.scale_denominator {
                geo_fields.push((format!("{prefix}Scale"), format!("1:{}", scale as u64)));
            }
            if let Some(unit) = &geo.unit_name {
                geo_fields.push((format!("{prefix}Unit"), unit.clone()));
            }
        }
        let geo_table = iced::widget::table(
            [
                iced::widget::table::column(
                    "Property",
                    |row: (String, String)| -> Element<'_, Message> {
                        text(row.0)
                            .size(14)
                            .font(INTER_BOLD)
                            .style(|_| iced::widget::text::Style {
                                color: Some(Color::from_rgb(0.5, 0.5, 0.5)),
                            })
                            .into()
                    },
                ),
                iced::widget::table::column(
                    "Value",
                    |row: (String, String)| -> Element<'_, Message> {
                        text(row.1).size(16).font(INTER_REGULAR).into()
                    },
                ),
            ],
            geo_fields,
        );
        column![
            text("\u{1f4cd}  Geospatial Data")
                .size(18)
                .font(INTER_BOLD)
                .style(|_| iced::widget::text::Style {
                    color: Some(Color::from_rgb8(80, 220, 120)),
                }),
            geo_table,
        ]
        .spacing(10)
        .into()
    };

    container(scrollable(
        container(column![header_row, meta_table, color_section, geo_section].spacing(24))
            .width(Length::Fixed(640.0))
            .padding(30)
            .style(|_| iced::widget::container::Style {
                background: Some(Color::from_rgb8(43, 45, 49).into()),
                border: iced::Border {
                    radius: 12.0.into(),
                    width: 1.0,
                    color: Color::from_rgb8(60, 60, 65),
                },
                ..Default::default()
            }),
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .style(|_| iced::widget::container::Style {
        background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.7).into()),
        ..Default::default()
    })
    .into()
}
