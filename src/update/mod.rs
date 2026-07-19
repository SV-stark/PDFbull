pub mod annotations;
pub mod app;
pub mod bookmarks;
pub mod export;
pub mod misc;
pub mod navigation;
pub mod render;
pub mod search;
pub mod tabs;

use crate::app::PdfBullApp;
use crate::message::Message;
use crate::models::AppTheme;
use crate::storage;
use iced::Task;

pub fn scroll_to_page(tab: &crate::models::DocumentTab, page: usize) -> Task<Message> {
    let y_offset: f32 = tab
        .page_heights
        .iter()
        .take(page)
        .map(|h| (h + crate::ui::theme::PAGE_SPACING) * tab.zoom)
        .sum();
    iced::widget::operation::scroll_to(
        "pdf_scroll",
        iced::widget::scrollable::AbsoluteOffset {
            x: 0.0,
            y: y_offset,
        },
    )
}

pub fn scroll_to_search_result(
    tab: &crate::models::DocumentTab,
    result_idx: usize,
) -> Task<Message> {
    if let Some(result) = tab.search_results.get(result_idx) {
        let page = result.page;
        let y_page_start: f32 = tab
            .page_heights
            .iter()
            .take(page)
            .map(|h| (h + crate::ui::theme::PAGE_SPACING) * tab.zoom)
            .sum();

        let actual_page = tab.page_mapping.get(page).copied().unwrap_or(page);
        let page_rotation = tab
            .page_rotations
            .get(&actual_page)
            .copied()
            .unwrap_or(tab.rotation);
        let original_height = tab.page_heights.get(page).copied().unwrap_or(800.0);

        let (_, ry, _, _) = crate::models::rotate_coords(
            result.x,
            result.y_position,
            result.width,
            result.height,
            tab.page_width,
            original_height,
            page_rotation,
        );

        let target_y = y_page_start + (ry * tab.zoom) - 100.0;
        let clamped_y = target_y.max(0.0);

        iced::widget::operation::scroll_to(
            "pdf_scroll",
            iced::widget::scrollable::AbsoluteOffset {
                x: 0.0,
                y: clamped_y,
            },
        )
    } else {
        Task::none()
    }
}

pub fn scroll_to_y(y_offset: f32) -> Task<Message> {
    iced::widget::operation::scroll_to(
        "pdf_scroll",
        iced::widget::scrollable::AbsoluteOffset {
            x: 0.0,
            y: y_offset,
        },
    )
}

pub fn handle_message(app: &mut PdfBullApp, message: Message) -> Task<Message> {
    if !app.loaded {
        app.loaded = true;
        app.settings = storage::load_settings();
        app.recent_files = storage::load_recent_files();
        let session = storage::load_session();
        if app.settings.theme == AppTheme::System {
            if matches!(dark_light::detect(), Ok(dark_light::Mode::Dark)) {
                app.settings.theme = AppTheme::Dark;
            } else {
                app.settings.theme = AppTheme::Light;
            }
        }
        let args: Vec<String> = std::env::args().collect();
        let mut cli_path = None;
        if args.len() > 1 {
            let path = std::path::PathBuf::from(&args[1]);
            if path.exists() && path.is_file() {
                cli_path = Some(path);
            }
        }

        let mut tasks = Vec::new();
        if let Some(path) = cli_path {
            tasks.push(app.update(Message::OpenFile(path)));
        } else if app.settings.restore_session
            && let Some(mut session_data) = session
        {
            let target_tab = session_data.active_tab;
            for entry in session_data.open_tabs.drain(..) {
                let path: std::path::PathBuf = entry.clone().into();
                tasks.push(app.update(Message::OpenFile(path)));
                if let crate::models::SessionTabEntry::Detailed(detailed) = entry {
                    if let Some(tab) = app.tabs.last_mut() {
                        tab.pending_session = Some(detailed);
                    }
                }
            }
            if !tasks.is_empty() {
                tasks.push(Task::perform(
                    async move { Message::SwitchTab(target_tab) },
                    |m| m,
                ));
            }
        }

        if !tasks.is_empty() {
            tasks.push(app.update(message));
            return Task::batch(tasks);
        }
    }

    match message {
        Message::ResetZoom
        | Message::OpenSettings
        | Message::CloseSettings
        | Message::SaveSettings(_)
        | Message::ToggleSidebar
        | Message::ToggleFormsSidebar
        | Message::ToggleFullscreen
        | Message::ToggleKeyboardHelp
        | Message::RotateClockwise
        | Message::RotateCounterClockwise
        | Message::ToggleMetadata
        | Message::SetSidebarMode(_)
        | Message::SetReadingMode(_)
        | Message::SetAnnotationColor(_)
        | Message::SetAnnotationThickness(_)
        | Message::SetAnnotationTextSize(_)
        | Message::ToggleMarkupBar
        | Message::ToggleTableMode
        | Message::ClearRecentFiles => app::handle_app_message(app, message),
        Message::AddBookmark | Message::RemoveBookmark(_) | Message::JumpToBookmark(_) => {
            bookmarks::handle_bookmark_message(app, message)
        }
        Message::AnnotationTextChanged(text) => {
            app.annotation_text = text;
            Task::none()
        }
        Message::SetAnnotationMode(_)
        | Message::AnnotationDragStart { .. }
        | Message::AnnotationDragUpdate { .. }
        | Message::AnnotationDragEnd
        | Message::DeleteAnnotation(_)
        | Message::Undo
        | Message::Redo
        | Message::SaveAnnotations
        | Message::AnnotationsSaved(_)
        | Message::AnnotationsLoaded(_, _)
        | Message::EditAnnotationText(_, _) => annotations::handle_annotation_message(app, message),
        Message::SetFilter(_)
        | Message::ToggleAutoCrop
        | Message::ViewportChanged(_, _)
        | Message::SidebarViewportChanged(_)
        | Message::RequestRender(_)
        | Message::PageRendered(_, _, _, _)
        | Message::ThumbnailRendered(_, _, _, _)
        | Message::TextItemsLoaded(_, _, _)
        | Message::TablesDetected(_, _, _) => render::handle_render_message(app, message),
        Message::OpenDocument
        | Message::DocumentOpenedWithPath(_)
        | Message::DocumentOpened(_, _)
        | Message::DocumentMetaLoaded(_, _)
        | Message::OpenFile(_)
        | Message::OpenRecentFile(_)
        | Message::CloseTab(_)
        | Message::SwitchTab(_)
        | Message::TabReordered(_)
        | Message::DocumentModifiedExternally(_)
        | Message::ReloadDocument(_)
        | Message::PasswordInputChanged(_)
        | Message::SubmitPassword
        | Message::CancelPasswordPrompt
        | Message::SaveAttachment(_)
        | Message::AttachmentSaved(_)
        | Message::ToggleLayer(_, _)
        | Message::LayerToggled => tabs::handle_tab_message(app, message),
        Message::NextPage
        | Message::PrevPage
        | Message::ZoomIn
        | Message::ZoomOut
        | Message::SetZoom(_)
        | Message::JumpToPage(_)
        | Message::PageInputChanged(_)
        | Message::PageInputSubmitted => navigation::handle_nav_message(app, message),
        Message::Search(_)
        | Message::PerformSearch(_)
        | Message::SearchResult(_, _)
        | Message::NextSearchResult
        | Message::PrevSearchResult
        | Message::ClearSearch => search::handle_search_message(app, message),
        Message::ExtractText
        | Message::ExtractTextToClipboard
        | Message::TextExtracted(_)
        | Message::CopyToClipboard(_)
        | Message::CopyImageToClipboard
        | Message::ExportImage
        | Message::ImageExported(_)
        | Message::ExportImages
        | Message::SaveOrganizedPDF
        | Message::OrganizedPDFSaved(_)
        | Message::Print
        | Message::ListPrinters
        | Message::PrintersListed(_)
        | Message::PrintWithPrinter(_)
        | Message::PrintDone(_)
        | Message::AddWatermark(_)
        | Message::WatermarkDone(_)
        | Message::OptimizePDF
        | Message::PDFOptimized(_)
        | Message::MergeDocuments(_)
        | Message::DocumentsMerged(_)
        | Message::SplitPDF(_)
        | Message::PDFSplit(_)
        | Message::LoadFormFields
        | Message::FormFieldsLoaded(_)
        | Message::FormFieldChanged(_, _)
        | Message::FillForm(_)
        | Message::FormFilled(_) => export::handle_export_message(app, message),
        Message::ToggleWatermarkPrompt(show) => {
            app.show_watermark_prompt = show;
            if show {
                app.show_signature_creator = false;
                app.show_page_organizer = false;
            }
            Task::none()
        }
        Message::ToggleSignaturesDetail(show) => {
            app.show_signatures_detail = show;
            Task::none()
        }
        Message::WatermarkInputChanged(input) => {
            app.watermark_input = input;
            Task::none()
        }
        Message::SubmitWatermark => {
            app.show_watermark_prompt = false;
            let text = app.watermark_input.clone();
            app.update(Message::AddWatermark(text))
        }
        Message::ToggleSignatureCreator(show) => {
            app.show_signature_creator = show;
            if show {
                app.show_watermark_prompt = false;
                app.show_page_organizer = false;
                app.signature_lines.clear();
                app.signature_drag = None;
            }
            Task::none()
        }
        Message::SignatureDragStart { x, y } => {
            app.signature_drag = Some((x, y));
            app.signature_lines.push(vec![(x, y)]);
            Task::none()
        }
        Message::SignatureDragUpdate { x, y } => {
            if app.signature_drag.is_some() {
                if let Some(line) = app.signature_lines.last_mut() {
                    line.push((x, y));
                }
                app.signature_drag = Some((x, y));
            }
            Task::none()
        }
        Message::SignatureDragEnd => {
            app.signature_drag = None;
            Task::none()
        }
        Message::ClearSignature => {
            app.signature_lines.clear();
            app.signature_drag = None;
            Task::none()
        }
        Message::SaveSignature => {
            if !app.signature_lines.is_empty() {
                app.saved_signature = Some(app.signature_lines.clone());
                app.signature_stamp_active = true;
                app.annotation_mode = Some(crate::models::PendingAnnotationKind::Line);
                app.status_message = Some(
                    "Signature saved. Click anywhere on the PDF page to stamp it.".to_string(),
                );
            }
            app.show_signature_creator = false;
            Task::none()
        }
        Message::TogglePageOrganizer(show) => {
            app.show_page_organizer = show;
            if show {
                app.show_watermark_prompt = false;
                app.show_signature_creator = false;
            }
            Task::none()
        }
        Message::OrganizerDeletePage(page_idx) => {
            if let Some(tab) = app.current_tab_mut() {
                if page_idx < tab.page_mapping.len() {
                    let actual_page = tab.page_mapping[page_idx];
                    tab.page_rotations.remove(&actual_page);
                    tab.page_mapping.remove(page_idx);
                    if page_idx < tab.page_heights.len() {
                        tab.page_heights.remove(page_idx);
                    }
                    // Remove annotations on the deleted page and shift remaining ones
                    tab.annotations.retain(|ann| ann.page != page_idx);
                    for ann in &mut tab.annotations {
                        if ann.page > page_idx {
                            ann.page -= 1;
                        }
                    }
                    tab.total_pages = tab.page_mapping.len();
                    tab.current_page = tab.current_page.min(tab.total_pages.saturating_sub(1));
                    tab.view_state.rendered_pages.clear();
                    tab.view_state.thumbnails.clear();
                }
            }
            app.render_visible_pages()
        }
        Message::OrganizerRotatePage(page_idx, rotation_diff) => {
            if let Some(tab) = app.current_tab_mut() {
                if page_idx < tab.page_mapping.len() {
                    let actual_page = tab.page_mapping[page_idx];
                    let current_rot = tab.page_rotations.entry(actual_page).or_insert(0);
                    *current_rot = (*current_rot + rotation_diff) % 360;
                    if *current_rot < 0 {
                        *current_rot += 360;
                    }
                    tab.view_state.rendered_pages.clear();
                    tab.view_state.thumbnails.clear();
                }
            }
            app.render_visible_pages()
        }
        Message::OrganizerMovePage(page_idx, direction) => {
            if let Some(tab) = app.current_tab_mut() {
                if let Ok(target_idx) = usize::try_from(page_idx as isize + direction) {
                    if page_idx < tab.page_mapping.len() && target_idx < tab.page_mapping.len() {
                        tab.page_mapping.swap(page_idx, target_idx);
                        if page_idx < tab.page_heights.len() && target_idx < tab.page_heights.len()
                        {
                            tab.page_heights.swap(page_idx, target_idx);
                        }
                        // Swap annotation page indices to keep them bound to the physical page content
                        for ann in &mut tab.annotations {
                            if ann.page == page_idx {
                                ann.page = target_idx;
                            } else if ann.page == target_idx {
                                ann.page = page_idx;
                            }
                        }
                        tab.view_state.rendered_pages.clear();
                        tab.view_state.thumbnails.clear();
                    }
                }
            }
            app.render_visible_pages()
        }
        Message::EngineInitialized(_)
        | Message::Error(_)
        | Message::ClearStatus
        | Message::IcedEvent(_)
        | Message::LinkClicked(_)
        | Message::ForceQuit => misc::handle_misc_message(app, message),
    }
}
