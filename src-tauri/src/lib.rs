use pdfium_render::prelude::*;
use slint::{Image, SharedPixelBuffer};
use std::sync::{Mutex, OnceLock};

struct PdfiumWrapper(pub Pdfium);
unsafe impl Sync for PdfiumWrapper {}
unsafe impl Send for PdfiumWrapper {}

static PDFIUM: OnceLock<Result<PdfiumWrapper, String>> = OnceLock::new();

fn get_pdfium_internal() -> Result<&'static Pdfium, String> {
    let result = PDFIUM.get_or_init(|| {
        println!("[PDF Engine] Initializing PDFium...");

        let path_result =
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
                .map_err(|e| format!("Current dir bind error: {:?}", e));

        let path_result = path_result.or_else(|_| {
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name())
                .map_err(|e| format!("System library bind error: {:?}", e))
        });

        match path_result {
            Ok(bindings) => {
                println!("[PDF Engine] PDFium library successfully bound.");
                Ok(PdfiumWrapper(Pdfium::new(bindings)))
            }
            Err(e) => {
                let err_msg = format!("PDFium library load failed: {}", e);
                eprintln!("[PDF Engine] {}", err_msg);
                Err(err_msg)
            }
        }
    });

    match result {
        Ok(wrapper) => Ok(&wrapper.0),
        Err(e) => Err(e.clone()),
    }
}

struct AppState {
    current_path: Option<String>,
    current_page: i32,
    total_pages: i32,
    zoom: f32,
}

static APP_STATE: OnceLock<Mutex<AppState>> = OnceLock::new();

fn get_state() -> &'static Mutex<AppState> {
    APP_STATE.get_or_init(|| {
        Mutex::new(AppState {
            current_path: None,
            current_page: 0,
            total_pages: 0,
            zoom: 1.0,
        })
    })
}

slint::include_modules!();

pub fn run() {
    println!("PDFbull starting - Pure Slint Edition");

    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("PANIC: {}", panic_info);
    }));

    let app = AppWindow::new().expect("Failed to create window");
    let app2 = app.clone_strong();
    let app3 = app2.clone_strong();
    let app4 = app3.clone_strong();
    let app5 = app4.clone_strong();
    let app6 = app5.clone_strong();
    let app7 = app6.clone_strong();
    let app8 = app7.clone_strong();

    app.on_open_file(move || {
        if let Some(file_path) = rfd::FileDialog::new()
            .add_filter("PDF Files", &["pdf"])
            .pick_file()
        {
            let path = file_path.to_string_lossy().to_string();
            println!("Opening: {}", path);

            match open_pdf(&path) {
                Ok(page_count) => {
                    let mut state = get_state().lock().unwrap();
                    state.current_path = Some(path.clone());
                    state.current_page = 0;
                    state.total_pages = page_count as i32;
                    state.zoom = 1.0;
                    drop(state);

                    app2.set_current_page(0);
                    app2.set_total_pages(page_count as i32);
                    app2.set_document_path(path.into());
                    app2.set_status_text(format!("Loaded - {} pages", page_count).into());
                    app2.set_zoom(1.0);
                }
                Err(e) => {
                    app2.set_status_text(format!("Error: {}", e).into());
                }
            }
        }
    });

    app3.on_close_file(move || {
        let mut state = get_state().lock().unwrap();
        state.current_path = None;
        state.current_page = 0;
        state.total_pages = 0;
        drop(state);

        app4.set_document_path("".into());
        app4.set_current_page(0);
        app4.set_total_pages(0);
        app4.set_status_text("Ready".into());
    });

    app5.on_previous_page(move || {
        let mut state = get_state().lock().unwrap();
        if state.current_page > 0 {
            state.current_page -= 1;
            let page = state.current_page;
            drop(state);
            app6.set_current_page(page);
        }
    });

    app7.on_next_page(move || {
        let mut state = get_state().lock().unwrap();
        if state.current_page < state.total_pages - 1 {
            state.current_page += 1;
            let page = state.current_page;
            drop(state);
            app8.set_current_page(page);
        }
    });

    let _ = app.run();
}

pub fn open_pdf(path: &str) -> Result<usize, String> {
    let pdfium = get_pdfium_internal()?;
    let doc = pdfium
        .load_pdf_from_file(path, None)
        .map_err(|e| format!("Failed to open PDF: {}", e))?;

    let page_count = doc.pages().len();

    Ok(page_count as usize)
}

fn render_page_to_image(path: &str, page_num: usize, scale: f32) -> Result<Image, String> {
    let pdfium = get_pdfium_internal()?;
    let doc = pdfium
        .load_pdf_from_file(path, None)
        .map_err(|e| format!("Failed to open PDF: {}", e))?;

    let page = doc
        .pages()
        .get(page_num as u16)
        .map_err(|e| format!("Failed to get page: {}", e))?;

    let width = (page.width().value * scale) as u32;
    let height = (page.height().value * scale) as u32;

    let bitmap = page
        .render(width as i32, height as i32, None)
        .map_err(|e| format!("Failed to render: {}", e))?;

    let raw_bytes = bitmap.as_raw_bytes();

    let mut buffer = SharedPixelBuffer::<slint::Rgb8Pixel>::new(width, height);

    {
        let slice = buffer.make_mut_bytes();
        for (i, pixel) in raw_bytes.chunks(4).enumerate() {
            if i * 3 + 2 < slice.len() {
                slice[i * 3] = pixel[2];
                slice[i * 3 + 1] = pixel[1];
                slice[i * 3 + 2] = pixel[0];
            }
        }
    }

    Ok(Image::from_rgb8(buffer))
}
