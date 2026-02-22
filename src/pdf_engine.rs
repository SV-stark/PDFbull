use moka::sync::Cache;
use pdfium_render::prelude::*;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
        auto_crop: bool,
    ) -> Result<(u32, u32, Arc<Vec<u8>>), String> {
        // Higher precision key: scale * 10000 + rotation + auto_crop
        let scale_key = (scale * 10000.0) as u32;
        let rotation_key = ((rotation + 360) % 360) as u32;
        let filter_key = filter as u32;
        let crop_key = if auto_crop { 1 } else { 0 };
        let cache_key = (
            page_num,
            scale_key * 100 + rotation_key + filter_key * 1000,
            crop_key,
        );

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

            let (target_w, target_h, crop_offset) = if auto_crop {
                // Render at higher resolution first to detect content
                let hi_scale = scale * 2.0;
                let hi_w = (page.width().value * hi_scale) as i32;
                let hi_h = (page.height().value * hi_scale) as i32;

                let hi_config = PdfRenderConfig::new()
                    .set_target_width(hi_w)
                    .set_maximum_height(hi_h)
                    .rotate(render_rotation, false);

                let hi_bitmap = page
                    .render_with_config(&hi_config)
                    .map_err(|e| e.to_string())?;

                // Find content bounding box
                let bbox = Self::detect_content_bbox(
                    &hi_bitmap.as_rgba_bytes(),
                    hi_bitmap.width() as u32,
                    hi_bitmap.height() as u32,
                );

                if let Some((x1, y1, x2, y2)) = bbox {
                    let crop_w = ((x2 - x1) as f32 / hi_scale * scale) as i32;
                    let crop_h = ((y2 - y1) as f32 / hi_scale * scale) as i32;
                    let offset_x = (x1 as f32 / hi_scale * scale) as i32;
                    let offset_y = (y1 as f32 / hi_scale * scale) as i32;
                    (crop_w.max(100), crop_h.max(100), Some((offset_x, offset_y)))
                } else {
                    (
                        (page.width().value * scale) as i32,
                        (page.height().value * scale) as i32,
                        None,
                    )
                }
            } else {
                (
                    (page.width().value * scale) as i32,
                    (page.height().value * scale) as i32,
                    None,
                )
            };

            let render_config = PdfRenderConfig::new()
                .set_target_width(target_w)
                .set_maximum_height(target_h)
                .rotate(render_rotation, false);

            let bitmap = page
                .render_with_config(&render_config)
                .map_err(|e| e.to_string())?;

            let w = bitmap.width() as u32;
            let h = bitmap.height() as u32;

            let rgba_data = bitmap.as_rgba_bytes().to_vec();
            let filtered_data = Self::apply_filter(rgba_data, w, h, filter);

            // Apply crop offset if auto_crop was used
            let result_data = if let Some((ox, oy)) = crop_offset {
                if ox > 0 || oy > 0 {
                    let mut cropped = vec![255u8; (w * h * 4) as usize];
                    for y in 0..h.min(oy as u32) {
                        for x in 0..w {
                            let src_idx = ((y as usize) * w as usize + x as usize) * 4;
                            let dst_idx = ((y as usize) * w as usize + x as usize) * 4;
                            if dst_idx < cropped.len() {
                                cropped[dst_idx] = 255;
                                cropped[dst_idx + 1] = 255;
                                cropped[dst_idx + 2] = 255;
                                cropped[dst_idx + 3] = 255;
                            }
                        }
                    }
                    cropped
                } else {
                    filtered_data
                }
            } else {
                filtered_data
            };

            let result_data = Arc::new(result_data);
            let result = (w, h, result_data);

            self.page_cache.insert(cache_key, result.clone());

            Ok(result)
        } else {
            Err("No active document".to_string())
        }
    }

    fn detect_content_bbox(data: &[u8], width: u32, height: u32) -> Option<(u32, u32, u32, u32)> {
        let w = width as usize;
        let h = height as usize;
        let threshold: u8 = 250;

        let mut min_x = w;
        let mut min_y = h;
        let mut max_x = 0usize;
        let mut max_y = 0usize;

        // Scan for non-white pixels (content)
        for y in 0..h {
            for x in 0..w {
                let idx = (y * w + x) * 4;
                if idx + 2 < data.len() {
                    let r = data[idx];
                    let g = data[idx + 1];
                    let b = data[idx + 2];
                    // Check if pixel is not white-ish
                    if r < threshold || g < threshold || b < threshold {
                        min_x = min_x.min(x);
                        min_y = min_y.min(y);
                        max_x = max_x.max(x);
                        max_y = max_y.max(y);
                    }
                }
            }
        }

        if max_x > min_x && max_y > min_y {
            // Add small margin
            let margin = 10;
            Some((
                min_x.saturating_sub(margin) as u32,
                min_y.saturating_sub(margin) as u32,
                (max_x + margin).min(w - 1) as u32,
                (max_y + margin).min(h - 1) as u32,
            ))
        } else {
            None
        }
    }

    fn apply_filter(mut data: Vec<u8>, _width: u32, _height: u32, filter: RenderFilter) -> Vec<u8> {
        if filter == RenderFilter::None {
            return data;
        }

        match filter {
            RenderFilter::Grayscale => {
                data.par_chunks_exact_mut(4).for_each(|pixel| {
                    let gray =
                        (pixel[0] as u32 * 299 + pixel[1] as u32 * 587 + pixel[2] as u32 * 114)
                            / 1000;
                    pixel[0] = gray as u8;
                    pixel[1] = gray as u8;
                    pixel[2] = gray as u8;
                });
            }
            RenderFilter::Inverted => {
                data.par_chunks_exact_mut(4).for_each(|pixel| {
                    pixel[0] = 255 - pixel[0];
                    pixel[1] = 255 - pixel[1];
                    pixel[2] = 255 - pixel[2];
                });
            }
            RenderFilter::Eco => {
                data.par_chunks_exact_mut(4).for_each(|pixel| {
                    let avg = ((pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32) / 3) as u8;
                    let eco = (avg as u32 * 8 / 10) as u8;
                    pixel[0] = eco;
                    pixel[1] = eco;
                    pixel[2] = eco;
                });
            }
            RenderFilter::BlackWhite => {
                data.par_chunks_exact_mut(4).for_each(|pixel| {
                    let gray =
                        (pixel[0] as u32 * 299 + pixel[1] as u32 * 587 + pixel[2] as u32 * 114)
                            / 1000;
                    let bw = if gray > 128 { 255 } else { 0 };
                    pixel[0] = bw;
                    pixel[1] = bw;
                    pixel[2] = bw;
                });
            }
            RenderFilter::Lighten => {
                data.par_chunks_exact_mut(4).for_each(|pixel| {
                    let lighten = |c: u8| (c as u32 * 3 / 2).min(255) as u8;
                    pixel[0] = lighten(pixel[0]);
                    pixel[1] = lighten(pixel[1]);
                    pixel[2] = lighten(pixel[2]);
                });
            }
            RenderFilter::NoShadow => {
                data.par_chunks_exact_mut(4).for_each(|pixel| {
                    let avg = ((pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32) / 3) as u8;
                    if avg < 64 {
                        pixel[0] = (pixel[0] + 64).min(255);
                        pixel[1] = (pixel[1] + 64).min(255);
                        pixel[2] = (pixel[2] + 64).min(255);
                    }
                });
            }
            RenderFilter::None => {}
        }

        data
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
