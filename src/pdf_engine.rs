use moka::sync::Cache;
use pdfium_render::prelude::*;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use memmap2::Mmap;
use std::fs::File;

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RenderFilter {
    #[default]
    None,
    Grayscale,
    Inverted,
    Eco,
    BlackWhite,
    Lighten,
    NoShadow,
}

#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RenderQuality {
    Low,
    #[default]
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderOptions {
    pub scale: f32,
    pub rotation: i32,
    pub filter: RenderFilter,
    pub auto_crop: bool,
    pub quality: RenderQuality,
}

pub type SharedRenderCache = Cache<RenderCacheKey, (u32, u32, Arc<Vec<u8>>)>;

pub struct DocumentStore<'a> {
    pdfium: &'a Pdfium,
    documents: HashMap<String, DocumentState<'a>>,
    render_cache: SharedRenderCache,
}

struct DocumentState<'a> {
    doc: PdfDocument<'a>,
    path: String,
    // We keep the mmap alive here to ensure the byte slice remains valid
    _mmap: Option<Arc<Mmap>>,
}

#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct RenderCacheKey {
    doc_id: String,
    page: i32,
    scale_key: u32,
    rotation: u32,
    filter: u32,
    auto_crop: bool,
    quality: RenderQuality,
}

pub fn create_render_cache(cache_size: u64) -> SharedRenderCache {
    Cache::builder()
        .max_capacity(cache_size)
        .weigher(|_key, val: &(u32, u32, Arc<Vec<u8>>)| val.2.len() as u32)
        .build()
}

#[derive(Clone, Debug)]
pub struct Bookmark {
    pub title: String,
    pub page_index: u32,
    pub children: Vec<Bookmark>,
}

impl<'a> DocumentStore<'a> {
    pub fn new(pdfium: &'a Pdfium, cache: SharedRenderCache) -> Result<Self, String> {
        Ok(Self {
            pdfium,
            documents: HashMap::new(),
            render_cache: cache,
        })
    }

    pub fn open_document(&mut self, path: &str) -> Result<(String, usize, Vec<f32>, f32), String> {
        let file = File::open(path).map_err(|e| e.to_string())?;
        let mmap = unsafe { Mmap::map(&file).map_err(|e| e.to_string())? };
        let mmap_arc = Arc::new(mmap);

        // Load PDF from mmap slice for zero-copy.
        // SAFETY: The Arc<Mmap> is stored in DocumentState and will outlive the PdfDocument.
        let bytes: &[u8] = mmap_arc.as_ref();
        let static_bytes: &'static [u8] = unsafe { std::mem::transmute(bytes) };

        let doc = self
            .pdfium
            .load_pdf_from_byte_slice(static_bytes, None)
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
            _mmap: Some(mmap_arc),
        };
        self.documents.insert(path.to_string(), state);

        Ok((path.to_string(), page_count as usize, heights, max_width))
    }

    pub fn ensure_opened(&mut self, path: &str) -> Result<(), String> {
        if !self.documents.contains_key(path) {
            self.open_document(path)?;
        }
        Ok(())
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

    pub fn invalidate_render_cache(&mut self, doc_id: &str) {
        let target_id = doc_id.to_string();
        let _ = self
            .render_cache
            .invalidate_entries_if(move |k: &RenderCacheKey, _v| k.doc_id == target_id);
    }

    pub fn close_document(&mut self, doc_id: &str) {
        self.documents.remove(doc_id);
        self.invalidate_render_cache(doc_id);
    }

    pub fn render_page(
        &self,
        doc_id: &str,
        page_num: i32,
        options: RenderOptions,
    ) -> Result<(u32, u32, Arc<Vec<u8>>), String> {
        let scale_key = (options.scale * 10000.0) as u32;
        let rotation_key = ((options.rotation + 360) % 360) as u32;
        let filter_key = options.filter as u32;
        let crop_key = options.auto_crop;

        let cache_key = RenderCacheKey {
            doc_id: doc_id.to_string(),
            page: page_num,
            scale_key,
            rotation: rotation_key,
            filter: filter_key,
            auto_crop: crop_key,
            quality: options.quality,
        };

        if let Some(cached) = self.render_cache.get(&cache_key) {
            return Ok(cached);
        }

        let state = self
            .documents
            .get(doc_id)
            .ok_or_else(|| "Document not found or closed".to_string())?;

        let doc = &state.doc;

        if page_num < 0 || page_num as usize >= doc.pages().len() as usize {
            return Err("Page number out of bounds".to_string());
        }

        let page = doc.pages().get(page_num).map_err(|e| e.to_string())?;

        let render_rotation = match ((options.rotation + 360) % 360) / 90 {
            0 => PdfPageRenderRotation::None,
            1 => PdfPageRenderRotation::Degrees90,
            2 => PdfPageRenderRotation::Degrees180,
            3 => PdfPageRenderRotation::Degrees270,
            _ => PdfPageRenderRotation::None,
        };

        let target_w = (page.width().value * options.scale) as i32;
        let target_h = (page.height().value * options.scale) as i32;

        let render_config = PdfRenderConfig::new()
            .set_target_width(target_w)
            .set_maximum_height(target_h)
            .rotate(render_rotation, false);

        let bitmap = page
            .render_with_config(&render_config)
            .map_err(|e| e.to_string())?;

        let w = bitmap.width() as u32;
        let h = bitmap.height() as u32;

        let result_data = Self::apply_filter(bitmap.as_rgba_bytes().to_vec(), w, h, options.filter);

        let (final_w, final_h, final_data) = if options.auto_crop {
            if let Some((x1, y1, x2, y2)) = Self::detect_content_bbox(&result_data, w, h) {
                let crop_w = (x2 - x1) + 1;
                let crop_h = (y2 - y1) + 1;
                let mut cropped = Vec::with_capacity((crop_w * crop_h * 4) as usize);
                for y in y1..=y2 {
                    let start = ((y * w + x1) * 4) as usize;
                    let end = ((y * w + x2 + 1) * 4) as usize;
                    cropped.extend_from_slice(&result_data[start..end]);
                }
                (crop_w.max(100), crop_h.max(100), cropped)
            } else {
                (w, h, result_data)
            }
        } else {
            (w, h, result_data)
        };

        let result_data: Arc<Vec<u8>> = Arc::new(final_data);
        let result = (final_w, final_h, result_data.clone());

        self.render_cache.insert(cache_key, result.clone());

        Ok(result)
    }

    fn detect_content_bbox(data: &[u8], width: u32, height: u32) -> Option<(u32, u32, u32, u32)> {
        let w = width as usize;
        let h = height as usize;
        let threshold: u8 = 250;

        let is_bg = |c: &[u8]| c[0] >= threshold && c[1] >= threshold && c[2] >= threshold;

        let mut min_y = h;
        for y in 0..h {
            let row_start = y * w * 4;
            let row = &data[row_start..row_start + w * 4];
            if !row.chunks_exact(4).all(is_bg) {
                min_y = y;
                break;
            }
        }

        if min_y == h {
            return None;
        }

        let mut max_y = 0;
        for y in (min_y..h).rev() {
            let row_start = y * w * 4;
            let row = &data[row_start..row_start + w * 4];
            if !row.chunks_exact(4).all(is_bg) {
                max_y = y;
                break;
            }
        }

        let mut min_x = w;
        for x in 0..w {
            let mut empty = true;
            for y in min_y..=max_y {
                let idx = (y * w + x) * 4;
                if !is_bg(&data[idx..idx + 4]) {
                    empty = false;
                    break;
                }
            }
            if !empty {
                min_x = x;
                break;
            }
        }

        let mut max_x = 0;
        for x in (min_x..w).rev() {
            let mut empty = true;
            for y in min_y..=max_y {
                let idx = (y * w + x) * 4;
                if !is_bg(&data[idx..idx + 4]) {
                    empty = false;
                    break;
                }
            }
            if !empty {
                max_x = x;
                break;
            }
        }

        if max_x >= min_x && max_y >= min_y {
            let margin = 10;
            Some((
                (min_x as u32).saturating_sub(margin),
                (min_y as u32).saturating_sub(margin),
                (max_x as u32 + margin).min(width.saturating_sub(1)),
                (max_y as u32 + margin).min(height.saturating_sub(1)),
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
                        pixel[0] = pixel[0].saturating_add(64);
                        pixel[1] = pixel[1].saturating_add(64);
                        pixel[2] = pixel[2].saturating_add(64);
                    }
                });
            }
            RenderFilter::None => {}
        }

        data
    }

    pub fn extract_text(&self, doc_id: &str, page_num: i32) -> Result<String, String> {
        let state = self
            .documents
            .get(doc_id)
            .ok_or_else(|| "Document not found or closed".to_string())?;

        let doc = &state.doc;

        if page_num < 0 || page_num as usize >= doc.pages().len() as usize {
            return Err("Page number out of bounds".to_string());
        }

        let page = doc.pages().get(page_num).map_err(|e| e.to_string())?;

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
        let state = self
            .documents
            .get(doc_id)
            .ok_or_else(|| "Document not found or closed".to_string())?;

        let doc = &state.doc;

        if page_num < 0 || page_num as usize >= doc.pages().len() as usize {
            return Err("Page number out of bounds".to_string());
        }

        let page = doc.pages().get(page_num).map_err(|e| e.to_string())?;

        let render_config = PdfRenderConfig::new()
            .set_target_width((page.width().value * scale) as i32)
            .set_maximum_height((page.height().value * scale) as i32)
            .rotate(PdfPageRenderRotation::None, false);

        let bitmap = page
            .render_with_config(&render_config)
            .map_err(|e| e.to_string())?;

        let img = bitmap.as_image().map_err(|e| e.to_string())?;
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
        let state = self
            .documents
            .get(doc_id)
            .ok_or_else(|| "Document not found or closed".to_string())?;

        let doc = &state.doc;

        let doc_name = state
            .path
            .rsplit(['/', '\\'])
            .next()
            .and_then(|s| s.strip_suffix(".pdf").or(Some(s)))
            .unwrap_or("document");

        let mut exported_paths = Vec::new();

        for (idx, &page_num) in page_nums.iter().enumerate() {
            if page_num < 0 || page_num as usize >= doc.pages().len() as usize {
                continue;
            }

            let page = match doc.pages().get(page_num) {
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

            let img = bitmap.as_image().map_err(|e| e.to_string())?;
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
        &mut self,
        doc_path: &str,
        annotations: &[crate::models::Annotation],
    ) -> Result<String, String> {
        let state = self
            .documents
            .iter_mut()
            .find(|(_, d)| d.path == doc_path)
            .map(|(_, d)| d)
            .ok_or_else(|| "Document not found".to_string())?;

        let doc = &state.doc;

        for ann in annotations {
            if ann.page >= doc.pages().len() as usize {
                continue;
            }
            let mut page = doc.pages().get(ann.page as i32).map_err(|e| e.to_string())?;
            
            match &ann.style {
                crate::models::AnnotationStyle::Highlight { color: _ } => {
                    let pdf_x = ann.x as f32;
                    let pdf_y = (page.height().value - ann.y - ann.height) as f32;
                    let pdf_w = ann.width as f32;
                    let pdf_h = ann.height as f32;

                    let rect = PdfRect::new(
                        PdfPoints::new(pdf_x),
                        PdfPoints::new(pdf_y),
                        PdfPoints::new(pdf_x + pdf_w),
                        PdfPoints::new(pdf_y + pdf_h),
                    );
                    
                    let page_annotations = page.annotations_mut();
                    if let Ok(mut pdf_ann) = page_annotations.create_highlight_annotation() {
                        let _ = pdf_ann.set_bounds(rect);
                    }
                }
                crate::models::AnnotationStyle::Rectangle { color: _, .. } => {
                    let pdf_x = ann.x as f32;
                    let pdf_y = (page.height().value - ann.y - ann.height) as f32;
                    let pdf_w = ann.width as f32;
                    let pdf_h = ann.height as f32;

                    let rect = PdfRect::new(
                        PdfPoints::new(pdf_x),
                        PdfPoints::new(pdf_y),
                        PdfPoints::new(pdf_x + pdf_w),
                        PdfPoints::new(pdf_y + pdf_h),
                    );
                    
                    let page_annotations = page.annotations_mut();
                    if let Ok(mut pdf_ann) = page_annotations.create_square_annotation() {
                        let _ = pdf_ann.set_bounds(rect);
                    }
                }
                _ => {}
            }
        }

        doc.save_to_file(doc_path).map_err(|e| e.to_string())?;
        Ok(doc_path.to_string())
    }

    pub fn load_annotations(
        &self,
        _doc_path: &str,
    ) -> Result<Vec<crate::models::Annotation>, String> {
        // Since we are now using native annotations, they are loaded automatically
        // when the PDF is opened. However, the app might expect them translated to its model.
        // For now, we'll return empty or potentially implement a translator if needed.
        // The user's app seems to have its own internal state management for annotations.
        Ok(Vec::new())
    }

    pub fn search(
        &self,
        doc_id: &str,
        query: &str,
    ) -> Result<Vec<crate::models::SearchResultItem>, String> {
        let state = self
            .documents
            .get(doc_id)
            .ok_or_else(|| "Document not found or closed".to_string())?;

        let doc = &state.doc;
        let mut results = Vec::new();
        let query_lower = query.to_lowercase();
        for (idx, page) in doc.pages().iter().enumerate() {
            if let Ok(text) = page.text() {
                let all_text = text.all();
                if all_text.to_lowercase().contains(&query_lower) {
                    results.push((
                        idx,
                        all_text,
                        0.0, 0.0, 100.0, 100.0, // Placeholder for now to ensure compilation
                    ));
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
        self.render_page(
            doc_id,
            page_num,
            RenderOptions {
                scale,
                rotation: 0,
                filter: RenderFilter::None,
                auto_crop: false,
                quality: RenderQuality::Low,
            },
        )
    }
}
