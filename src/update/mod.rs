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
        if app.settings.restore_session
            && let Some(mut session_data) = session
        {
            let target_tab = session_data.active_tab;
            let mut tasks = Vec::new();
            for path in session_data.open_tabs.drain(..) {
                tasks.push(app.update(Message::OpenFile(path.into())));
            }
            if !tasks.is_empty() {
                tasks.push(Task::perform(
                    async move { Message::SwitchTab(target_tab) },
                    |m| m,
                ));
                tasks.push(app.update(message));
                return Task::batch(tasks);
            }
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
        | Message::ClearRecentFiles => app::handle_app_message(app, message),
        Message::AddBookmark | Message::RemoveBookmark(_) | Message::JumpToBookmark(_) => {
            bookmarks::handle_bookmark_message(app, message)
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
        | Message::AnnotationsLoaded(_, _) => annotations::handle_annotation_message(app, message),
        Message::SetFilter(_)
        | Message::ToggleAutoCrop
        | Message::ViewportChanged(_, _)
        | Message::SidebarViewportChanged(_)
        | Message::RequestRender(_)
        | Message::PageRendered(_, _, _)
        | Message::ThumbnailRendered(_, _, _) => render::handle_render_message(app, message),
        Message::OpenDocument
        | Message::DocumentOpenedWithPath(_)
        | Message::DocumentOpened(_)
        | Message::OpenFile(_)
        | Message::OpenRecentFile(_)
        | Message::CloseTab(_)
        | Message::SwitchTab(_)
        | Message::TabReordered(_)
        | Message::DocumentModifiedExternally(_)
        | Message::ReloadDocument(_) => tabs::handle_tab_message(app, message),
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
        | Message::Print
        | Message::PrintDone(_)
        | Message::AddWatermark(_)
        | Message::WatermarkDone(_)
        | Message::MergeDocuments(_)
        | Message::DocumentsMerged(_)
        | Message::SplitPDF(_)
        | Message::PDFSplit(_)
        | Message::LoadFormFields
        | Message::FormFieldsLoaded(_)
        | Message::FormFieldChanged(_, _)
        | Message::FillForm(_)
        | Message::FormFilled(_) => export::handle_export_message(app, message),
        Message::EngineInitialized(_)
        | Message::Error(_)
        | Message::ClearStatus
        | Message::IcedEvent(_)
        | Message::LinkClicked(_)
        | Message::ForceQuit => misc::handle_misc_message(app, message),
    }
}
