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

pub struct DocumentStore {
    pdfium: Pdfium,
    documents: Cache<String, DocumentState>,
    render_cache: Cache<RenderCacheKey, (u32, u32, Arc<Vec<u8>>)>,
}

struct DocumentState {
    doc: PdfDocument<'static>,
    path: String,
}

#[derive(Clone, Hash, Eq, PartialEq)]
struct RenderCacheKey {
    doc_id: String,
    page: i32,
    scale_key: u32,
    rotation: u32,
    filter: u32,
    auto_crop: bool,
}

#[derive(Clone, Debug)]
pub struct Bookmark {
    pub title: String,
    pub page_index: u32,
    pub children: Vec<Bookmark>,
}

impl DocumentStore {
    pub fn new() -> Result<Self, String> {
        let bindings = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
            .or_else(|_| Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name()))
            .map_err(|e| format!("Failed to bind to Pdfium library: {}", e))?;

        let pdfium = Pdfium::new(bindings);

        Ok(Self {
            pdfium,
            documents: Cache::builder()
                .max_capacity(10)
                .time_to_idle(Duration::from_secs(300))
                .build(),
            render_cache: Cache::builder()
                .max_capacity(100)
                .time_to_idle(Duration::from_secs(300))
                .build(),
        })
    }

    pub fn open_document(&mut self, path: &str) -> Result<(String, usize, Vec<f32>, f32), String> {
        let doc_id = path.to_string();
        
        let doc = self
            .pdfium
            .load_pdf_from_file(path, None)
            .map_err(|e| e.to_string())?;

        let pages = doc.pages();
        let page_count = pages.len();

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

        let state = DocumentState {
            doc,
            path: path.to_string(),
        };
        self.documents.insert(doc_id.clone(), state);

        self.invalidate_render_cache(&doc_id);

        Ok((path.to_string(), page_count as usize, heights, max_width))
    }

    pub fn get_outline(&self, doc_id: &str) -> Vec<Bookmark> {
        if let Some(state) = self.documents.get(doc_id) {
            Self::extract_bookmarks_internal(state.doc.bookmarks().root().as_ref())
        } else {
            vec![]
        }
    }

    fn extract_bookmarks_internal(bookmark: Option<&PdfBookmark>) -> Vec<Bookmark> {
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

            item.children = Self::extract_bookmarks_internal(Some(&bm));
            result.push(item);

            current = bm.next_sibling();
        }
        result
    }

    fn invalidate_render_cache(&self, doc_id: &str) {
        let keys_to_remove: Vec<_> = self.render_cache
            .keys()
            .filter(|k| k.doc_id == doc_id)
            .collect();
        for key in keys_to_remove {
            self.render_cache.invalidate(&key);
        }
    }

    pub fn close_document(&self, doc_id: &str) {
        self.documents.invalidate(doc_id);
        self.invalidate_render_cache(doc_id);
    }

    pub fn render_page(
        &self,
        doc_id: &str,
        page_num: i32,
        scale: f32,
        rotation: i32,
        filter: RenderFilter,
        auto_crop: bool,
    ) -> Result<(u32, u32, Arc<Vec<u8>>), String> {
        let scale_key = (scale * 10000.0) as u32;
        let rotation_key = ((rotation + 360) % 360) as u32;
        let filter_key = filter as u32;
        let crop_key = auto_crop;
        
        let cache_key = RenderCacheKey {
            doc_id: doc_id.to_string(),
            page: page_num,
            scale_key,
            rotation: rotation_key,
            filter: filter_key,
            auto_crop: crop_key,
        };

        if let Some(cached) = self.render_cache.get(&cache_key) {
            return Ok(cached);
        }

        let state = self.documents.get(doc_id)
            .ok_or_else(|| "Document not found or closed".to_string())?;

        let doc = &state.doc;
        
        if page_num < 0 || page_num as usize >= doc.pages().len() {
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

        let mut result_data = Self::apply_filter(bitmap.as_rgba_bytes().to_vec(), w, h, filter);

        if let Some((ox, oy)) = crop_offset {
            if ox > 0 || oy > 0 {
                let crop_y = oy.min(h as i32) as u32;
                for y in 0..crop_y {
                    let row_start = (y * w) as usize * 4;
                    let row_end = row_start + (w as usize * 4);
                    if row_end <= result_data.len() {
                        result_data[row_start..row_end].fill(255);
                    }
                }
            }
        }

        let result_data = Arc::new(result_data);
        let result = (w, h, result_data.clone());

        self.render_cache.insert(cache_key, result.clone());

        Ok(result)
    }

    fn detect_content_bbox(data: &[u8], width: u32, height: u32) -> Option<(u32, u32, u32, u32)> {
        let w = width as usize;
        let h = height as usize;
        let threshold: u8 = 250;

        let mut min_x = w;
        let mut min_y = h;
        let mut max_x = 0usize;
        let mut max_y = 0usize;

        for y in 0..h {
            for x in 0..w {
                let idx = (y * w + x) * 4;
                if idx + 2 < data.len() {
                    let r = data[idx];
                    let g = data[idx + 1];
                    let b = data[idx + 2];
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

    pub fn extract_text(&self, doc_id: &str, page_num: i32) -> Result<String, String> {
        let state = self.documents.get(doc_id)
            .ok_or_else(|| "Document not found or closed".to_string())?;

        let doc = &state.doc;

        if page_num < 0 || page_num as usize >= doc.pages().len() {
            return Err("Page number out of bounds".to_string());
        }

        let page = doc
            .pages()
            .get(page_num as u16)
            .map_err(|e| e.to_string())?;

        let text = page.text().map_err(|e| e.to_string())?;

        Ok(text.to_string())
    }

    pub fn export_page_as_image(
        &self,
        doc_id: &str,
        page_num: i32,
        scale: f32,
        path: &str,
    ) -> Result<(), String> {
        let state = self.documents.get(doc_id)
            .ok_or_else(|| "Document not found or closed".to_string())?;

        let doc = &state.doc;

        if page_num < 0 || page_num as usize >= doc.pages().len() {
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
    }

    pub fn export_pages_as_images(
        &self,
        doc_id: &str,
        page_nums: &[i32],
        scale: f32,
        output_dir: &str,
    ) -> Result<Vec<String>, String> {
        let state = self.documents.get(doc_id)
            .ok_or_else(|| "Document not found or closed".to_string())?;

        let doc = &state.doc;
        
        let doc_name = state.path
            .rsplit(['/', '\\'])
            .next()
            .and_then(|s| s.strip_suffix(".pdf").or(Some(s)))
            .unwrap_or("document");

        let mut exported_paths = Vec::new();

        for (idx, &page_num) in page_nums.iter().enumerate() {
            if page_num < 0 || page_num as usize >= doc.pages().len() {
                continue;
            }

            let page = match doc.pages().get(page_num as u16) {
                Ok(p) => p,
                Err(_) => continue,
            };

            let render_config = PdfRenderConfig::new()
                .set_target_width((page.width().value * scale) as i32)
                .set_maximum_height((page.height().value * scale) as i32)
                .rotate(PdfPageRenderRotation::None, false);

            let bitmap = match page.render_with_config(&render_config) {
                Ok(b) => b,
                Err(_) => continue,
            };

            let img = bitmap.as_image();
            let rgba = match img.as_rgba8() {
                Some(i) => i,
                None => continue,
            };

            let filename = format!("{}_page{}_{}.png", doc_name, page_num, idx);
            let path = std::path::Path::new(output_dir).join(&filename);

            if let Err(e) = rgba.save(&path) {
                eprintln!("Failed to save page {}: {}", page_num, e);
                continue;
            }

            exported_paths.push(path.to_string_lossy().to_string());
        }

        Ok(exported_paths)
    }

    pub fn save_annotations(
        &self,
        doc_path: &str,
        annotations: &[crate::models::Annotation],
    ) -> Result<String, String> {
        let pdf_path_obj = std::path::Path::new(doc_path);
        let annotation_path = pdf_path_obj.with_extension("annotations.json");

        let json = serde_json::to_string_pretty(annotations)
            .map_err(|e| format!("Failed to serialize annotations: {}", e))?;

        std::fs::write(&annotation_path, json)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    format!("Cannot save annotations: directory is read-only. Annotations not saved.")
                } else {
                    format!("Failed to write annotations file: {}", e)
                }
            })?;

        Ok(annotation_path.to_string_lossy().to_string())
    }

    pub fn load_annotations(
        &self,
        doc_path: &str,
    ) -> Result<Vec<crate::models::Annotation>, String> {
        let pdf_path_obj = std::path::Path::new(doc_path);
        let annotation_path = pdf_path_obj.with_extension("annotations.json");

        if !annotation_path.exists() {
            return Ok(Vec::new());
        }

        let json = std::fs::read_to_string(&annotation_path)
            .map_err(|e| format!("Failed to read annotations file: {}", e))?;

        serde_json::from_str(&json).map_err(|e| format!("Failed to parse annotations: {}", e))
    }

    pub fn search(
        &self,
        doc_id: &str,
        query: &str,
    ) -> Result<Vec<(usize, String, f32)>, String> {
        let state = self.documents.get(doc_id)
            .ok_or_else(|| "Document not found or closed".to_string())?;

        let doc = &state.doc;
        let mut results = Vec::new();
        let query_lower = query.to_lowercase();

        for (idx, page) in doc.pages().iter().enumerate() {
            if let Ok(text) = page.text() {
                let text_str = text.to_string();
                if text_str.to_lowercase().contains(&query_lower) {
                    let result = (idx, text_str.chars().take(200).collect(), 0.0);
                    results.push(result);
                }
            }
        }

        Ok(results)
    }

    pub fn get_thumbnail(
        &self,
        doc_id: &str,
        page_num: i32,
        scale: f32,
    ) -> Result<(u32, u32, Arc<Vec<u8>>), String> {
        self.render_page(doc_id, page_num, scale, 0, RenderFilter::None, false)
    }
}
