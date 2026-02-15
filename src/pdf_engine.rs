use pdfium_render::prelude::*;
use std::sync::OnceLock;

struct PdfiumWrapper(Pdfium);
unsafe impl Send for PdfiumWrapper {}
unsafe impl Sync for PdfiumWrapper {}

static PDFIUM: OnceLock<Result<PdfiumWrapper, String>> = OnceLock::new();

fn get_pdfium() -> Result<&'static Pdfium, String> {
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
                let err_msg = format!(
                    "PDFium library load failed: {}. Ensure pdfium.dll is in resources or root.",
                    e
                );
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

pub struct PdfState {
    active_doc: Option<PdfDocument<'static>>,
    current_path: Option<String>,
}

impl PdfState {
    pub fn new() -> Self {
        Self {
            active_doc: None,
            current_path: None,
        }
    }

    pub fn close_document(&mut self) {
        self.active_doc = None;
        self.current_path = None;
    }

    pub fn open_document(&mut self, path: &str) -> Result<i32, String> {
        let pdfium = get_pdfium()?;
        let doc = pdfium
            .load_pdf_from_file(path, None)
            .map_err(|e| format!("Failed to open PDF: {}", e))?;

        let page_count = doc.pages().len() as i32;
        self.active_doc = Some(doc);
        self.current_path = Some(path.to_string());

        Ok(page_count)
    }

    pub fn render_page(&self, page_num: i32, scale: f32) -> Result<(u32, u32, Vec<u8>), String> {
        let doc = self.active_doc.as_ref().ok_or("No active document")?;
        let page = doc
            .pages()
            .get(page_num as u16)
            .map_err(|e| e.to_string())?;

        let width = (page.width().value * scale) as i32;
        let height = (page.height().value * scale) as i32;

        let bitmap = page
            .render(width, height, None)
            .map_err(|e| e.to_string())?;

        Ok((width as u32, height as u32, bitmap.as_raw_bytes().to_vec()))
    }
}
