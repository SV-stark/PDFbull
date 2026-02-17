use moka::sync::Cache;
use pdfium_render::prelude::*;
use std::sync::Arc;
use std::time::Duration;

pub struct PdfEngine<'a> {
    pdfium: &'a Pdfium,
    active_doc: Option<PdfDocument<'a>>,
    // Cache key: (page_index, scale_key) -> (width, height, rgba_data)
    // Scale stored as u32 (scale * 10000) to be hashable and precise
    page_cache: Cache<(i32, u32), (u32, u32, Arc<Vec<u8>>)>,
}

#[derive(Clone, Debug)]
pub struct Bookmark {
    pub title: String,
    pub page_index: u32,
    pub children: Vec<Bookmark>,
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
            // Moka cache with weighted size (bytes) or capacity
            // Let's use a generous capacity for now, effectively "unlimited" relative to user navigation
            // but evicted by time-to-live or max capacity if needed.
            // 50 pages @ 4K is ~1.5GB. Moka handles this well.
            page_cache: Cache::builder()
                .max_capacity(50)
                .time_to_idle(Duration::from_secs(300)) // 5 min unused -> evict
                .build(),
        }
    }

    pub fn close_document(&mut self) {
        self.active_doc = None;
        self.page_cache.invalidate_all();
    }

    pub fn open_document(&mut self, path: &str) -> Result<(usize, Vec<f32>, f32), String> {
        self.page_cache.invalidate_all();

        // Load document
        let doc = self
            .pdfium
            .load_pdf_from_file(path, None)
            .map_err(|e| e.to_string())?;

        let pages = doc.pages();
        let page_count = pages.len();

        // Calculate dimensions
        let mut heights = Vec::with_capacity(page_count as usize);
        let mut max_width = 0.0;

        for i in 0..page_count {
            if let Ok(page) = pages.get(i) {
                let w = page.width().value;
                let h = page.height().value;
                heights.push(h);
                if w > max_width {
                    max_width = w;
                }
            } else {
                heights.push(0.0);
            }
        }

        self.active_doc = Some(doc);

        Ok((page_count as usize, heights, max_width))
    }

    pub fn get_outline(&self) -> Vec<Bookmark> {
        if let Some(doc) = &self.active_doc {
            self.extract_bookmarks(&doc.bookmarks())
        } else {
            vec![]
        }
    }

    fn extract_bookmarks(&self, bookmarks: &PdfBookmarks) -> Vec<Bookmark> {
        let mut result = Vec::new();
        for bookmark in bookmarks.iter() {
            let mut item = Bookmark {
                title: bookmark.title(),
                page_index: 0,
                children: Vec::new(),
            };

            if let Some(dest) = bookmark.destination() {
                item.page_index = dest.page_index();
            }

            item.children = self.extract_bookmarks(&bookmark.children());
            result.push(item);
        }
        result
    }

    pub fn render_page(
        &self,
        page_num: i32,
        scale: f32,
    ) -> Result<(u32, u32, Arc<Vec<u8>>), String> {
        // Higher precision key: 1.25 -> 12500
        let scale_key = (scale * 10000.0) as u32;
        let cache_key = (page_num, scale_key);

        if let Some(cached) = self.page_cache.get(&cache_key) {
            return Ok(cached);
        }

        if let Some(doc) = &self.active_doc {
            // Fix u16 overflow: cast to usize directly (assuming PDFium supports it, but len() returns u16 in wrapper usually)
            // pdfium-render uses u16 for paging in some versions, let's check safety.
            // doc.pages() returns PdfPages which has .len() -> u16.
            // So if PDF > 65535 pages, we have a bigger problem with the library wrapper.
            // But let's just properly check bounds against len()
            if page_num < 0 || page_num as usize >= doc.pages().len() as usize {
                return Err("Page number out of bounds".to_string());
            }

            let page = doc
                .pages()
                .get(page_num as u16)
                .map_err(|e| e.to_string())?;

            let render_config = PdfRenderConfig::new()
                .set_target_width((page.width().value * scale) as u32)
                .set_maximum_height((page.height().value * scale) as u32)
                .rotate(PdfPageRenderRotation::None, false);

            // Render to bitmap (BGRA)
            let bitmap = page
                .render_with_config(&render_config)
                .map_err(|e| e.to_string())?;

            let w = bitmap.width() as u32;
            let h = bitmap.height() as u32;

            // Optimize: Convert BGRA -> RGBA
            let mut bytes = bitmap.as_raw_bytes().to_vec();

            // Optimized BGRA -> RGBA conversion
            for chunk in bytes.chunks_exact_mut(4) {
                chunk.swap(0, 2); // Swap B and R
            }

            let result_data = Arc::new(bytes);
            let result = (w, h, result_data);

            // Store in cache
            self.page_cache.insert(cache_key, result.clone());

            Ok(result)
        } else {
            Err("No active document".to_string())
        }
    }
}
