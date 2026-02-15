use pdfium_render::prelude::*;
use slint::{Image, SharedPixelBuffer};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

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

pub struct PdfWrapper(pub PdfDocument<'static>);
unsafe impl Send for PdfWrapper {}
unsafe impl Sync for PdfWrapper {}

pub struct PdfState {
    pub docs: Arc<Mutex<HashMap<String, PdfWrapper>>>,
    pub active_doc: Arc<Mutex<Option<String>>>,
}

impl PdfState {
    pub fn new() -> Self {
        Self {
            docs: Arc::new(Mutex::new(HashMap::new())),
            active_doc: Arc::new(Mutex::new(None)),
        }
    }
}

slint::include_modules!();

pub fn run() {
    println!("PDFbull starting - Pure Slint Edition");

    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("PANIC: {}", panic_info);
    }));

    let app = AppWindow::new().expect("Failed to create window");

    let h1 = app.clone_strong();
    let h2 = h1.clone_strong();
    let h3 = h2.clone_strong();
    let h4 = h3.clone_strong();
    let h5 = h4.clone_strong();
    let h6 = h5.clone_strong();
    let h7 = h6.clone_strong();
    let h8 = h7.clone_strong();
    let h9 = h8.clone_strong();
    let h10 = h9.clone_strong();

    h1.on_open_file(move || {
        if let Some(file_path) = rfd::FileDialog::new()
            .add_filter("PDF Files", &["pdf"])
            .pick_file()
        {
            let path = file_path.to_string_lossy().to_string();
            println!("Opening: {}", path);

            match open_pdf(&path) {
                Ok(page_count) => {
                    h2.set_current_page(0);
                    h2.set_total_pages(page_count as i32);
                    h2.set_document_path(path.into());
                    h2.set_status_text("Document loaded".into());
                }
                Err(e) => {
                    h2.set_status_text(format!("Error: {}", e).into());
                }
            }
        }
    });

    h3.on_previous_page(move || {
        let c = h4.get_current_page();
        if c > 0 {
            h4.set_current_page(c - 1);
        }
    });

    h5.on_next_page(move || {
        let c = h6.get_current_page();
        let t = h6.get_total_pages();
        if c < t - 1 {
            h6.set_current_page(c + 1);
        }
    });

    h7.on_zoom_in(move || {
        let c = h8.get_zoom();
        let nz = (c + 0.25).min(5.0);
        h8.set_zoom(nz);
    });

    app.run();
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
