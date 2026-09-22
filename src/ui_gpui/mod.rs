pub mod app_view;
pub mod canvas;
pub mod dialogs;
pub mod log_console;
pub mod ribbon;
pub mod sidebar;
pub mod tabs;
pub mod welcome;

use gpui_kit::component::{ActiveTheme, Root};
use gpui_kit::*;

pub fn run_gpui_app() {
    let initial_file = std::env::args().skip(1).find(|arg| {
        !arg.starts_with('-')
            && (arg.to_lowercase().ends_with(".pdf") || std::path::Path::new(arg).exists())
    });

    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);

            let initial_file_clone = initial_file.clone();
            cx.spawn(async move |cx| {
                cx.open_window(WindowOptions::default(), |window, cx| {
                    let view = cx.new(|cx| {
                        let mut v = app_view::PdfbullView::new(window, cx);
                        if let Some(ref path) = initial_file_clone {
                            v.open_pdf_path(path, cx);
                        }
                        v
                    });
                    cx.new(|cx| {
                        let bg = cx.theme().background;
                        Root::new(view, window, cx).bg(bg)
                    })
                })
                .expect("Failed to open PDFbull window");
            })
            .detach();
        });
}
