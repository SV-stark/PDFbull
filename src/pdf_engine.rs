use crate::models::{Annotation, AnnotationStyle, Hyperlink, SearchResultItem, PdfError, PdfResult};
use lru::LruCache;
use pdfium_render::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use rayon::prelude::*;

use crate::ui::theme::hex_to_rgb;

pub struct RenderCache {
    lru: LruCache<String, crate::models::RenderResult>,
    /// Maps (path, page_num) -> last used cache key to allow evicting old scales of the same page
    scale_index: HashMap<(String, usize), String>,
    current_bytes: usize,
    max_bytes: usize,
}

impl RenderCache {
    pub fn new(capacity: usize, max_bytes: usize) -> Self {
        Self {
            lru: LruCache::new(
                std::num::NonZeroUsize::new(capacity)
                    .unwrap_or(std::num::NonZeroUsize::new(1).unwrap()),
            ),
            scale_index: HashMap::new(),
            current_bytes: 0,
            max_bytes,
        }
    }

    pub fn get(&mut self, key: &str) -> Option<crate::models::RenderResult> {
        self.lru.get(key).cloned()
    }

    pub fn put(
        &mut self,
        path: &str,
        page_num: usize,
        key: String,
        result: crate::models::RenderResult,
    ) {
        let entry_bytes = result.data.len();

        let page_key = (path.to_string(), page_num);
        if let Some(old_key) = self.scale_index.insert(page_key.clone(), key.clone()) {
            if old_key != key {
                if let Some(old_res) = self.lru.pop(&old_key) {
                    self.current_bytes = self.current_bytes.saturating_sub(old_res.data.len());
                }
            }
        }

        while self.current_bytes + entry_bytes > self.max_bytes && self.lru.len() > 0 {
            if let Some((_, v)) = self.lru.pop_lru() {
                self.current_bytes = self.current_bytes.saturating_sub(v.data.len());
            }
        }

        self.lru.put(key, result);
        self.current_bytes += entry_bytes;
    }
}

pub type SharedRenderCache = Arc<Mutex<RenderCache>>;

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum RenderQuality {
    Low,
    Medium,
    High,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum RenderFilter {
    None,
    Grayscale,
    Inverted,
    Eco,
    BlackWhite,
    Lighten,
    NoShadow,
}

#[derive(Debug, Clone)]
pub struct RenderOptions {
    pub scale: f32,
    pub rotation: i32,
    pub filter: RenderFilter,
    pub auto_crop: bool,
    pub quality: RenderQuality,
}

pub struct DocumentStore<'a> {
    pdfium: &'a Pdfium,
    documents: HashMap<String, DocumentState<'a>>,
    render_cache: SharedRenderCache,
}

struct DocumentState<'a> {
    doc: PdfDocument<'a>,
}

impl<'a> DocumentStore<'a> {
    pub fn new(pdfium: &'a Pdfium, cache: SharedRenderCache) -> PdfResult<Self> {
        Ok(Self {
            pdfium,
            documents: HashMap::new(),
            render_cache: cache,
        })
    }

    pub fn open_document(&mut self, path: &str, doc_id: crate::models::DocumentId) -> PdfResult<crate::models::OpenResult> {
        let doc = self
            .pdfium
            .load_pdf_from_file(path, None)
            .map_err(|e| PdfError::OpenFailed(e.to_string()))?;

        let pages = doc.pages();
        let page_count = pages.len();

        let mut heights = Vec::with_capacity(page_count as usize);
        let mut max_width = 0.0;
        let mut all_links = Vec::new();

        for i in 0..page_count {
            if let Ok(page) = pages.get(i as i32) {
                let w = page.width().value;
                let h = page.height().value;
                heights.push(h);
                if w > max_width {
                    max_width = w;
                }

                for link in page.links().iter() {
                    if let Ok(rect) = link.rect() {
                        let x = rect.left().value;
                        let y = rect.bottom().value;
                        let lw = rect.width().value;
                        let lh = rect.height().value;

                        let url = link
                            .action()
                            .and_then(|a| a.as_uri_action().and_then(|u| u.uri().ok()));
                        let dest = link
                            .destination()
                            .and_then(|d| d.page_index().ok())
                            .map(|idx| idx as usize);

                        if url.is_some() || dest.is_some() {
                            all_links.push(Hyperlink {
                                page: i as usize,
                                bounds: (x, y, lw, lh),
                                url,
                                destination_page: dest,
                            });
                        }
                    }
                }
            } else {
                heights.push(0.0);
            }
        }

        let outline = self.get_outline_internal(&doc);
        let state = DocumentState { doc };
        self.documents.insert(path.to_string(), state);

        Ok(crate::models::OpenResult {
            id: doc_id,
            page_count: page_count as usize,
            page_heights: heights,
            max_width,
            outline,
            links: all_links,
        })
    }

    pub fn ensure_opened(&mut self, path: &str, doc_id: crate::models::DocumentId) -> PdfResult<()> {
        if !self.documents.contains_key(path) {
            self.open_document(path, doc_id)?;
        }
        Ok(())
    }

    pub fn close_document(&mut self, path: &str) {
        self.documents.remove(path);
    }

    pub fn render_page(
        &mut self,
        path: &str,
        page_num: usize,
        options: RenderOptions,
    ) -> PdfResult<crate::models::RenderResult> {
        let rounded_scale = (options.scale * 100.0).round() / 100.0;
        let cache_key = format!(
            "{}_{}_{}_{:?}_{}_{:?}",
            path, page_num, rounded_scale, options.filter, options.auto_crop, options.quality
        );

        {
            let mut cache = self.render_cache.lock().map_err(|e| PdfError::EngineError(e.to_string()))?;
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(cached);
            }
        }

        let state = self
            .documents
            .get(path)
            .ok_or_else(|| PdfError::EngineError("Document not found or closed".to_string()))?;

        let doc = &state.doc;
        let page = doc
            .pages()
            .get(page_num as i32)
            .map_err(|_| PdfError::PageNotFound(page_num))?;

        let mut target_w = (page.width().value * rounded_scale) as i32;
        let mut target_h = (page.height().value * rounded_scale) as i32;

        let max_dim = 2500;
        if target_w > max_dim || target_h > max_dim {
            let scale_factor = max_dim as f32 / (target_w.max(target_h) as f32);
            target_w = (target_w as f32 * scale_factor) as i32;
            target_h = (target_h as f32 * scale_factor) as i32;
        }

        let render_rotation = match options.rotation {
            90 => PdfPageRenderRotation::Degrees90,
            180 => PdfPageRenderRotation::Degrees180,
            270 => PdfPageRenderRotation::Degrees270,
            _ => PdfPageRenderRotation::None,
        };

        let mut render_config = PdfRenderConfig::new()
            .set_target_width(target_w)
            .set_maximum_height(target_h)
            .rotate(render_rotation, false)
            .use_lcd_text_rendering(true);

        if options.filter == RenderFilter::Grayscale {
            render_config = render_config.use_grayscale_rendering(true);
        }

        let bitmap = page
            .render_with_config(&render_config)
            .map_err(|e| PdfError::RenderFailed(e.to_string()))?;

        let w = bitmap.width() as u32;
        let h = bitmap.height() as u32;

        let result_data =
            if options.filter == RenderFilter::None || options.filter == RenderFilter::Grayscale {
                bitmap.as_rgba_bytes().to_vec()
            } else {
                Self::apply_filter_parallel(bitmap.as_rgba_bytes().to_vec(), options.filter)
            };

        let (final_w, final_h, final_data) = if options.auto_crop {
            if let Some((x1, y1, x2, y2)) = Self::detect_content_bbox_parallel(&result_data, w, h) {
                let crop_w = (x2 - x1) + 1;
                let crop_h = (y2 - y1) + 1;
                let mut cropped = Vec::with_capacity((crop_w * crop_h * 4) as usize);
                for y in y1..=y2 {
                    let start = ((y * w + x1) * 4) as usize;
                    let end = ((y * w + x2 + 1) * 4) as usize;
                    cropped.extend_from_slice(&result_data[start..end]);
                }
                (crop_w, crop_h, cropped)
            } else {
                (w, h, result_data)
            }
        } else {
            (w, h, result_data)
        };

        let result = crate::models::RenderResult {
            width: final_w,
            height: final_h,
            data: Arc::new(final_data),
        };

        {
            let mut cache = self.render_cache.lock().map_err(|e| PdfError::EngineError(e.to_string()))?;
            cache.put(path, page_num, cache_key, result.clone());
        }

        Ok(result)
    }

    pub fn extract_text(&self, path: &str, page_num: i32) -> PdfResult<String> {
        let state = self
            .documents
            .get(path)
            .ok_or_else(|| PdfError::EngineError("Document not found".to_string()))?;
        let page = state.doc.pages().get(page_num).map_err(|_| PdfError::PageNotFound(page_num as usize))?;
        let text_page = page.text().map_err(|e| PdfError::SearchError(e.to_string()))?;
        Ok(text_page.all())
    }

    pub fn save_annotations(
        &mut self,
        pdf_path: &str,
        annotations: &[Annotation],
    ) -> PdfResult<String> {
        let state = self
            .documents
            .get_mut(pdf_path)
            .ok_or_else(|| PdfError::EngineError("Document not found".to_string()))?;
        let doc = &mut state.doc;

        for ann in annotations {
            let mut page = doc
                .pages()
                .get(ann.page as i32)
                .map_err(|_| PdfError::PageNotFound(ann.page))?;
            let page_height = page.height().value;
            let mut objects = page.objects_mut();

            let pdf_top = page_height - ann.y;
            let pdf_bottom = page_height - (ann.y + ann.height);
            let pdf_left = ann.x;
            let pdf_right = ann.x + ann.width;

            let rect = PdfRect::new(
                PdfPoints::new(pdf_top),
                PdfPoints::new(pdf_left),
                PdfPoints::new(pdf_bottom),
                PdfPoints::new(pdf_right),
            );

            match &ann.style {
                AnnotationStyle::Highlight { color } => {
                    let (r, g, b) = hex_to_rgb(color);
                    let fill_color = Some(PdfColor::new(
                        (r * 255.0) as u8,
                        (g * 255.0) as u8,
                        (b * 255.0) as u8,
                        100,
                    ));

                    let _ = objects
                        .create_path_object_rect(rect, None, None, fill_color)
                        .map_err(|e| PdfError::RenderFailed(e.to_string()))?;
                }
                AnnotationStyle::Rectangle { color, thickness, fill } => {
                    let (r, g, b) = hex_to_rgb(color);
                    let fill_color = if *fill {
                        Some(PdfColor::new((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, 50))
                    } else {
                        None
                    };

                    let stroke_color = Some(PdfColor::new((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, 255));

                    let _ = objects
                        .create_path_object_rect(rect, stroke_color, Some(PdfPoints::new(*thickness)), fill_color)
                        .map_err(|e| PdfError::RenderFailed(e.to_string()))?;
                }
                AnnotationStyle::Text { text, color, font_size } => {
                    let (r, g, b) = hex_to_rgb(color);
                    let font = doc.fonts_mut().helvetica();
                    let mut text_obj = objects
                        .create_text_object(PdfPoints::new(pdf_left), PdfPoints::new(pdf_bottom), text, font, PdfPoints::new(*font_size as f32))
                        .map_err(|e| PdfError::RenderFailed(e.to_string()))?;
                    text_obj.set_fill_color(PdfColor::new((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, 255))
                        .map_err(|e| PdfError::RenderFailed(e.to_string()))?;
                }
            }
        }

        let output_path = pdf_path.replace(".pdf", "_annotated.pdf");
        doc.save_to_file(&output_path).map_err(|e| PdfError::IoError(e.to_string()))?;
        Ok(output_path)
    }

    pub fn export_page_as_image(
        &self,
        path: &str,
        page_num: i32,
        scale: f32,
        output_path: &str,
    ) -> PdfResult<()> {
        let state = self
            .documents
            .get(path)
            .ok_or_else(|| PdfError::EngineError("Document not found".to_string()))?;
        let page = state.doc.pages().get(page_num).map_err(|_| PdfError::PageNotFound(page_num as usize))?;

        let render_config = PdfRenderConfig::new()
            .set_target_width((page.width().value * scale) as i32)
            .set_maximum_height((page.height().value * scale) as i32)
            .rotate(PdfPageRenderRotation::None, false)
            .use_lcd_text_rendering(true);

        let bitmap = page
            .render_with_config(&render_config)
            .map_err(|e| PdfError::RenderFailed(e.to_string()))?;
        let img = bitmap.as_image().map_err(|e| PdfError::RenderFailed(e.to_string()))?;
        img.save(output_path).map_err(|e| PdfError::IoError(e.to_string()))?;
        Ok(())
    }

    pub fn get_outline_internal(&self, doc: &PdfDocument) -> Vec<Bookmark> {
        let mut bookmarks = Vec::new();
        for b in doc.bookmarks().iter() {
            if let Some(title) = b.title() {
                bookmarks.push(Bookmark {
                    title,
                    page_index: b
                        .destination()
                        .and_then(|d| d.page_index().ok())
                        .unwrap_or(0) as u16,
                });
            }
        }
        bookmarks
    }

    pub fn search(&self, doc_id: &str, query: &str) -> PdfResult<Vec<SearchResultItem>> {
        let state = self
            .documents
            .get(doc_id)
            .ok_or_else(|| PdfError::EngineError("Document not found or closed".to_string()))?;

        let doc = &state.doc;
        let mut results = Vec::new();

        for (page_idx, page) in doc.pages().iter().enumerate() {
            if let Ok(text) = page.text() {
                let searcher = text.search(query, &PdfSearchOptions::new())
                    .map_err(|e| PdfError::SearchError(e.to_string()))?;
                    
                for segments in searcher.iter(PdfSearchDirection::SearchForward) {
                    let mut text_all = String::new();
                    let mut first_rect = None;

                    for segment in segments.iter() {
                        if first_rect.is_none() {
                            first_rect = Some(segment.bounds());
                        }
                        text_all.push_str(&segment.text());
                    }

                    if let Some(rect) = first_rect {
                        results.push(SearchResultItem {
                            page_index: page_idx,
                            text: text_all,
                            y: rect.bottom().value,
                            x: rect.left().value,
                            width: rect.width().value,
                            height: rect.height().value,
                        });
                    }
                }
            }
        }
        Ok(results)
    }

    fn detect_content_bbox_parallel(data: &[u8], width: u32, height: u32) -> Option<(u32, u32, u32, u32)> {
        let is_not_bg = |p: &[u8]| p[0] <= 245 || p[1] <= 245 || p[2] <= 245;

        let active_pixels: Vec<(u32, u32)> = data.par_chunks_exact(4)
            .enumerate()
            .filter_map(|(idx, pixel)| {
                if is_not_bg(pixel) {
                    let x = (idx as u32) % width;
                    let y = (idx as u32) / width;
                    Some((x, y))
                } else {
                    None
                }
            })
            .collect();

        if active_pixels.is_empty() { return None; }

        let min_x = active_pixels.iter().map(|p| p.0).min().unwrap();
        let max_x = active_pixels.iter().map(|p| p.0).max().unwrap();
        let min_y = active_pixels.iter().map(|p| p.1).min().unwrap();
        let max_y = active_pixels.iter().map(|p| p.1).max().unwrap();

        let margin = 10;
        Some((
            min_x.saturating_sub(margin),
            min_y.saturating_sub(margin),
            (max_x + margin).min(width.saturating_sub(1)),
            (max_y + margin).min(height.saturating_sub(1)),
        ))
    }

    fn apply_filter_parallel(mut data: Vec<u8>, filter: RenderFilter) -> Vec<u8> {
        match filter {
            RenderFilter::Inverted => {
                data.par_chunks_exact_mut(4).for_each(|pixel| {
                    pixel[0] = 255 - pixel[0];
                    pixel[1] = 255 - pixel[1];
                    pixel[2] = 255 - pixel[2];
                });
            }
            RenderFilter::Eco => {
                data.par_chunks_exact_mut(4).for_each(|pixel| {
                    let avg = (pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32) / 3;
                    if avg > 200 {
                        pixel[0] = 255; pixel[1] = 255; pixel[2] = 255;
                    }
                });
            }
            RenderFilter::BlackWhite => {
                data.par_chunks_exact_mut(4).for_each(|pixel| {
                    let avg = (pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32) / 3;
                    let val = if avg > 128 { 255 } else { 0 };
                    pixel[0] = val; pixel[1] = val; pixel[2] = val;
                });
            }
            RenderFilter::Lighten => {
                data.par_chunks_exact_mut(4).for_each(|pixel| {
                    pixel[0] = pixel[0].saturating_add(20);
                    pixel[1] = pixel[1].saturating_add(20);
                    pixel[2] = pixel[2].saturating_add(20);
                });
            }
            RenderFilter::NoShadow => {
                data.par_chunks_exact_mut(4).for_each(|pixel| {
                    if pixel[0] > 230 && pixel[1] > 230 && pixel[2] > 230 {
                        pixel[0] = 255; pixel[1] = 255; pixel[2] = 255;
                    }
                });
            }
            _ => {}
        }
        data
    }
}

pub fn create_render_cache(cache_size: u64, max_memory_mb: u64) -> SharedRenderCache {
    let max_bytes = (max_memory_mb * 1024 * 1024) as usize;
    Arc::new(Mutex::new(RenderCache::new(
        cache_size as usize,
        if max_bytes == 0 { 512 * 1024 * 1024 } else { max_bytes },
    )))
}

#[derive(Clone, Debug)]
pub struct Bookmark {
    pub title: String,
    pub page_index: u16,
}
