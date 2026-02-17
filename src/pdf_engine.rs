use pdfium_render::prelude::*;

pub struct PdfEngine<'a> {
    pdfium: &'a Pdfium,
    active_doc: Option<PdfDocument<'a>>,
    current_path: Option<String>,
}

impl<'a> PdfEngine<'a> {
    pub fn init_pdfium() -> Result<Pdfium, String> {
        let bindings = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
            .or_else(|_| Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name()))
            .map_err(|e| format!("Failed to bind to Pdfium library: {}", e))?;
        
        Ok(Pdfium::new(bindings))
    }

    pub fn new(pdfium: &'a Pdfium) -> Self {
        Self {
            pdfium,
            active_doc: None,
            current_path: None,
        }
    }

    pub fn close_document(&mut self) {
        self.active_doc = None;
        self.current_path = None;
    }

    pub fn open_document(&mut self, path: &str) -> Result<i32, String> {
        // Load the document
        // We need to ensure the document lives as long as 'a (the Pdfium instance)
        // load_pdf_from_file returns PdfDocument<'a> if created from Pdfium<'a>
        
        let doc = self.pdfium
            .load_pdf_from_file(path, None)
            .map_err(|e| e.to_string())?;

        let page_count = doc.pages().len();
        self.active_doc = Some(doc);
        self.current_path = Some(path.to_string());

        Ok(page_count as i32)
    }

    pub fn render_page(&self, page_num: i32, scale: f32) -> Result<(u32, u32, Vec<u8>), String> {
        if let Some(doc) = &self.active_doc {
            if page_num < 0 || page_num as u16 >= doc.pages().len() {
                return Err("Page number out of bounds".to_string());
            }

            let page = doc.pages().get(page_num as u16).map_err(|e| e.to_string())?;
            
            // Calculate dimensions
            let width = page.width().value;
            let height = page.height().value;
            
            let render_config = PdfRenderConfig::new()
                .scale_page_by_factor(scale)
                .set_target_width((width * scale) as i32)
                .set_target_height((height * scale) as i32)
                .rotate_if_landscape(PdfPageRenderRotation::None, true);

            let bitmap = page.render_with_config(&render_config).map_err(|e| e.to_string())?;
            
            // Get the buffer
            // The logic here depends on the bitmap format. Pdfium usually returns BGRA meaning B, G, R, A order
            
            let w = bitmap.width() as u32;
            let h = bitmap.height() as u32;
            let bytes = bitmap.as_raw_bytes().to_vec();

            Ok((w, h, bytes))
        } else {
            Err("No document loaded".to_string())
        }
    }
}
