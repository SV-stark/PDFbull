use moka::sync::Cache;
use pdfium_render::prelude::*;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderFilter {
    None,
    Grayscale,
    Inverted,
    Eco,
    BlackWhite,
    Lighten,
    NoShadow,
}

impl Default for RenderFilter {
    fn default() -> Self {
        RenderFilter::None
    }
}

pub struct PdfEngine<'a> {
    pdfium: &'a Pdfium,
    active_doc: Option<PdfDocument<'a>>,
    // Cache key: (page_index, scale_key, filter) -> (width, height, rgba_data)
    // Scale stored as u32 (scale * 10000) to be hashable and precise
    page_cache: Cache<(i32, u32, u32), (u32, u32, Arc<Vec<u8>>)>,
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
            self.extract_bookmarks(doc.bookmarks().root().as_ref())
        } else {
            vec![]
        }
    }

    fn extract_bookmarks(&self, bookmark: Option<&PdfBookmark>) -> Vec<Bookmark> {
        let mut result = Vec::new();

        let mut current = bookmark.and_then(|b| b.first_child());

        while let Some(bm) = current {
            let mut item = Bookmark {
                title: bm.title().unwrap_or_default(),
                page_index: 0,
                children: Vec::new(),
            };

            if let Some(dest) = bm.destination() {
                if let Ok(idx) = dest.page_index() {
                    item.page_index = idx as u32;
                }
            }

            item.children = self.extract_bookmarks(Some(&bm));
            result.push(item);

            current = bm.next_sibling();
        }
        result
    }

    pub fn render_page(
        &self,
        page_num: i32,
        scale: f32,
        rotation: i32,
        filter: RenderFilter,
    ) -> Result<(u32, u32, Arc<Vec<u8>>), String> {
        // Higher precision key: scale * 10000 + rotation
        let scale_key = (scale * 10000.0) as u32;
        let rotation_key = ((rotation + 360) % 360) as u32;
        let filter_key = filter as u32;
        let cache_key = (page_num, scale_key * 100 + rotation_key, filter_key);

        if let Some(cached) = self.page_cache.get(&cache_key) {
            return Ok(cached);
        }

        if let Some(doc) = &self.active_doc {
            if page_num < 0 || page_num as usize >= doc.pages().len() as usize {
                return Err("Page number out of bounds".to_string());
            }

            let page = doc
                .pages()
                .get(page_num as u16)
                .map_err(|e| e.to_string())?;

            let render_rotation = match ((rotation + 360) % 360) / 90 {
                0 => PdfPageRenderRotation::None,
                1 => PdfPageRenderRotation::Degrees90,
                2 => PdfPageRenderRotation::Degrees180,
                3 => PdfPageRenderRotation::Degrees270,
                _ => PdfPageRenderRotation::None,
            };

            let render_config = PdfRenderConfig::new()
                .set_target_width((page.width().value * scale) as i32)
                .set_maximum_height((page.height().value * scale) as i32)
                .rotate(render_rotation, false);

            let bitmap = page
                .render_with_config(&render_config)
                .map_err(|e| e.to_string())?;

            let w = bitmap.width() as u32;
            let h = bitmap.height() as u32;

            let rgba_data = bitmap.as_rgba_bytes().to_vec();
            let filtered_data = Self::apply_filter(rgba_data, w, h, filter);
            let result_data = Arc::new(filtered_data);
            let result = (w, h, result_data);

            self.page_cache.insert(cache_key, result.clone());

            Ok(result)
        } else {
            Err("No active document".to_string())
        }
    }

    fn apply_filter(data: Vec<u8>, width: u32, height: u32, filter: RenderFilter) -> Vec<u8> {
        if filter == RenderFilter::None {
            return data;
        }

        let mut result = data.clone();
        let total_pixels = (width * height) as usize;

        match filter {
            RenderFilter::Grayscale => {
                for i in (0..total_pixels * 4).step_by(4) {
                    let gray = (result[i] as u32 * 299
                        + result[i + 1] as u32 * 587
                        + result[i + 2] as u32 * 114)
                        / 1000;
                    result[i] = gray as u8;
                    result[i + 1] = gray as u8;
                    result[i + 2] = gray as u8;
                }
            }
            RenderFilter::Inverted => {
                for i in (0..total_pixels * 4).step_by(4) {
                    result[i] = 255 - result[i];
                    result[i + 1] = 255 - result[i + 1];
                    result[i + 2] = 255 - result[i + 2];
                }
            }
            RenderFilter::Eco => {
                for i in (0..total_pixels * 4).step_by(4) {
                    let avg = ((result[i] as u32 + result[i + 1] as u32 + result[i + 2] as u32) / 3)
                        as u8;
                    let eco = (avg as u32 * 8 / 10) as u8;
                    result[i] = eco;
                    result[i + 1] = eco;
                    result[i + 2] = eco;
                }
            }
            RenderFilter::BlackWhite => {
                for i in (0..total_pixels * 4).step_by(4) {
                    let gray = (result[i] as u32 * 299
                        + result[i + 1] as u32 * 587
                        + result[i + 2] as u32 * 114)
                        / 1000;
                    let bw = if gray > 128 { 255 } else { 0 };
                    result[i] = bw;
                    result[i + 1] = bw;
                    result[i + 2] = bw;
                }
            }
            RenderFilter::Lighten => {
                for i in (0..total_pixels * 4).step_by(4) {
                    let lighten = |c: u8| (c as u32 * 3 / 2).min(255) as u8;
                    result[i] = lighten(result[i]);
                    result[i + 1] = lighten(result[i + 1]);
                    result[i + 2] = lighten(result[i + 2]);
                }
            }
            RenderFilter::NoShadow => {
                for i in (0..total_pixels * 4).step_by(4) {
                    let avg = ((result[i] as u32 + result[i + 1] as u32 + result[i + 2] as u32) / 3)
                        as u8;
                    if avg < 64 {
                        result[i] = (result[i] + 64).min(255);
                        result[i + 1] = (result[i + 1] + 64).min(255);
                        result[i + 2] = (result[i + 2] + 64).min(255);
                    }
                }
            }
            RenderFilter::None => {}
        }

        result
    }

    pub fn extract_text(&self, page_num: i32) -> Result<String, String> {
        if let Some(doc) = &self.active_doc {
            if page_num < 0 || page_num as usize >= doc.pages().len() as usize {
                return Err("Page number out of bounds".to_string());
            }

            let page = doc
                .pages()
                .get(page_num as u16)
                .map_err(|e| e.to_string())?;

            let text = page.text().map_err(|e| e.to_string())?;

            Ok(text.to_string())
        } else {
            Err("No active document".to_string())
        }
    }

    pub fn export_page_as_image(
        &self,
        page_num: i32,
        scale: f32,
        path: &str,
    ) -> Result<(), String> {
        if let Some(doc) = &self.active_doc {
            if page_num < 0 || page_num as usize >= doc.pages().len() as usize {
                return Err("Page number out of bounds".to_string());
            }

            let page = doc
                .pages()
                .get(page_num as u16)
                .map_err(|e| e.to_string())?;

            let render_config = PdfRenderConfig::new()
                .set_target_width((page.width().value * scale) as i32)
                .set_maximum_height((page.height().value * scale) as i32)
                .rotate(PdfPageRenderRotation::None, false);

            let bitmap = page
                .render_with_config(&render_config)
                .map_err(|e| e.to_string())?;

            let img = bitmap.as_image();
            let rgba = img.as_rgba8();
            let image = match rgba {
                Some(i) => i,
                None => return Err("Failed to convert to RGBA8".to_string()),
            };

            image.save(path).map_err(|e| format!("{}", e))?;

            Ok(())
        } else {
            Err("No active document".to_string())
        }
    }

    pub fn search(&self, query: &str) -> Result<Vec<(usize, String, f32)>, String> {
        if let Some(doc) = &self.active_doc {
            let mut results = Vec::new();
            let query_lower = query.to_lowercase();

            for (idx, page) in doc.pages().iter().enumerate() {
                if let Ok(text) = page.text() {
                    let text_str = text.to_string();
                    if text_str.to_lowercase().contains(&query_lower) {
                        results.push((idx, text_str.chars().take(200).collect(), 0.0));
                    }
                }
            }

            Ok(results)
        } else {
            Err("No active document".to_string())
        }
    }
}
