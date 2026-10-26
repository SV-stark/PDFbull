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
        // `assets::Assets` embeds only gpui-kit's curated subset (its
        // `default-icons.txt`); the reader's icon set (House, PanelTopClose,
        // Rows2, Columns2, Merge, Signature, …) reaches past that subset, and
        // every missing glyph is a guaranteed panic at first paint. `AllAssets`
        // ships the full Lucide set instead.
        .with_assets(gpui_kit::assets::AllAssets)
        .run(move |cx| {
            gpui_kit::init(cx);

            let initial_file_clone = initial_file.clone();
            cx.spawn(async move |cx| {
                // Do not `expect` here: this runs inside a detached task, and a
                // panic in a detached task is swallowed rather than surfaced.
                // Report the failure and exit cleanly instead.
                let window_options = WindowOptions {
                    window_min_size: Some(gpui_kit::size(px(800.0), px(600.0))),
                    ..WindowOptions::default()
                };
                // The view is needed after the window exists, to hand it the
                // files a second launch forwards.
                let mut view_slot: Option<gpui_kit::Entity<app_view::PdfbullView>> = None;
                match cx.open_window(window_options, |window, cx| {
                    let view = cx.new(|cx| {
                        let mut v = app_view::PdfbullView::new(window, cx);
                        if let Some(ref path) = initial_file_clone {
                            v.open_pdf_path(path, cx);
                        }
                        v
                    });
                    view_slot = Some(view.clone());
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

                // A second launch does not start a second copy: it connects to
                // this process and sends its command line, which we open here.
                // Nothing drained the channel before, so those arguments were
                // received and thrown away.
                if let (Some(view), Some(mut forwarded)) =
                    (view_slot, crate::platform::take_forwarded_args())
                {
                    cx.spawn(async move |cx| {
                        while let Some(args) = forwarded.recv().await {
                            let paths: Vec<String> = args
                                .into_iter()
                                .skip(1)
                                .filter(|arg| {
                                    !arg.starts_with('-')
                                        && sidebar::SidebarState::is_openable_pdf(
                                            std::path::Path::new(arg.as_str()),
                                        )
                                })
                                .collect();
                            if paths.is_empty() {
                                continue;
                            }
                            view.update(cx, |view, cx| {
                                for path in &paths {
                                    view.open_pdf_path(path, cx);
                                }
                            });
                        }
                    })
                    .detach();
                }
            })
            .detach();
        });
}
