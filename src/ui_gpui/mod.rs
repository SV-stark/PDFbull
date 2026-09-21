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
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);

            cx.spawn(async move |cx| {
                cx.open_window(WindowOptions::default(), |window, cx| {
                    let view = cx.new(|cx| app_view::PdfbullView::new(window, cx));
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
