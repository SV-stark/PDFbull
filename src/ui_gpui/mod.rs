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
    // Only accept a real PDF file as the startup document. The previous test
    // (`... || Path::new(arg).exists()`) accepted directories and any existing
    // path, and `--version`-style flags were already handled in `main`.
    let initial_file = std::env::args().skip(1).find(|arg| {
        !arg.starts_with('-')
            && sidebar::SidebarState::is_openable_pdf(std::path::Path::new(arg.as_str()))
    });

    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);

            let initial_file_clone = initial_file.clone();
            cx.spawn(async move |cx| {
                // Do not `expect` here: this runs inside a detached task, and a
                // panic in a detached task is swallowed rather than surfaced.
                // Report the failure and exit cleanly instead.
                match cx.open_window(WindowOptions::default(), |window, cx| {
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
                }) {
                    Ok(_) => tracing::info!("PDFbull window created."),
                    Err(e) => {
                        tracing::error!("Failed to open the PDFbull window: {e}");
                        eprintln!("PDFbull: failed to open window: {e}");
                    }
                }
            })
            .detach();
        });
}
