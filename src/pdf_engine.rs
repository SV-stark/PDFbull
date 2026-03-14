use crate::models::{Annotation, AnnotationStyle, Hyperlink, SearchResultItem};
use lru::LruCache;
use pdfium_render::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

pub type SharedRenderCache = Arc<Mutex<LruCache<String, crate::models::RenderResult>>>;

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
    path: String,
}

impl<'a> DocumentStore<'a> {
    pub fn new(pdfium: &'a Pdfium, cache: SharedRenderCache) -> Result<Self, String> {
        Ok(Self {
            pdfium,
            documents: HashMap::new(),
            render_cache: cache,
        })
    }

    pub fn open_document(
        &mut self,
        path: &str,
    ) -> Result<(String, usize, Vec<f32>, f32, Vec<Hyperlink>), String> {
        let doc = self
            .pdfium
            .load_pdf_from_file(path, None)
            .map_err(|e| e.to_string())?;

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

                // Extract links
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

        let state = DocumentState {
            doc,
            path: path.to_string(),
        };

        self.documents.insert(path.to_string(), state);

        Ok((
            path.to_string(),
            page_count as usize,
            heights,
            max_width,
            all_links,
        ))
    }

    pub fn ensure_opened(&mut self, path: &str) -> Result<(), String> {
        if !self.documents.contains_key(path) {
            self.open_document(path)?;
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
    ) -> Result<crate::models::RenderResult, String> {
        let cache_key = format!(
            "{}_{}_{}_{:?}_{}_{:?}",
            path, page_num, options.scale, options.filter, options.auto_crop, options.quality
        );

        {
            let mut cache = self.render_cache.lock().map_err(|e| e.to_string())?;
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(cached.clone());
            }
        }

        let state = self
            .documents
            .get(path)
            .ok_or_else(|| "Document not found or closed".to_string())?;

        let doc = &state.doc;
        let page = doc
            .pages()
            .get(page_num as i32)
            .map_err(|e| e.to_string())?;

        let mut target_w = (page.width().value * options.scale) as i32;
        let mut target_h = (page.height().value * options.scale) as i32;

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
            .map_err(|e| e.to_string())?;

        let w = bitmap.width() as u32;
        let h = bitmap.height() as u32;

        let result_data =
            if options.filter == RenderFilter::None || options.filter == RenderFilter::Grayscale {
                bitmap.as_rgba_bytes().to_vec()
            } else {
                Self::apply_filter(bitmap.as_rgba_bytes().to_vec(), w, h, options.filter)
            };

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
                (crop_w, crop_h, cropped)
            } else {
                (w, h, result_data)
            }
        } else {
            (w, h, result_data)
        };

        let result_data: Arc<Vec<u8>> = Arc::new(final_data);
        let result = (final_w, final_h, result_data.clone());

        {
            let mut cache = self.render_cache.lock().map_err(|e| e.to_string())?;
            cache.put(cache_key, result.clone());
        }

        Ok(result)
    }

    pub fn extract_text(&self, path: &str, page_num: i32) -> Result<String, String> {
        let state = self
            .documents
            .get(path)
            .ok_or_else(|| "Document not found".to_string())?;
        let page = state.doc.pages().get(page_num).map_err(|e| e.to_string())?;
        let text_page = page.text().map_err(|e| e.to_string())?;
        Ok(text_page.all())
    }

    pub fn save_annotations(
        &mut self,
        pdf_path: &str,
        annotations: &[Annotation],
    ) -> Result<String, String> {
        let state = self
            .documents
            .get_mut(pdf_path)
            .ok_or_else(|| "Document not found".to_string())?;
        let doc = &mut state.doc;

        for ann in annotations {
            let mut page = doc
                .pages()
                .get(ann.page as i32)
                .map_err(|e| e.to_string())?;
            let mut objects = page.objects_mut();

            let rect = PdfRect::new(
                PdfPoints::new(ann.y + ann.height),
                PdfPoints::new(ann.x),
                PdfPoints::new(ann.y),
                PdfPoints::new(ann.x + ann.width),
            );

            match &ann.style {
                AnnotationStyle::Highlight { color } => {
                    let (r, g, b) = Self::hex_to_rgb(color);
                    let fill_color = Some(PdfColor::new(
                        (r * 255.0) as u8,
                        (g * 255.0) as u8,
                        (b * 255.0) as u8,
                        100,
                    ));

                    let _rect_obj = objects
                        .create_path_object_rect(rect, None, None, fill_color)
                        .map_err(|e| e.to_string())?;
                }
                AnnotationStyle::Rectangle {
                    color,
                    thickness,
                    fill,
                } => {
                    let (r, g, b) = Self::hex_to_rgb(color);
                    let fill_color = if *fill {
                        Some(PdfColor::new(
                            (r * 255.0) as u8,
                            (g * 255.0) as u8,
                            (b * 255.0) as u8,
                            50,
                        ))
                    } else {
                        None
                    };
                    let stroke_color = Some(PdfColor::new(
                        (r * 255.0) as u8,
                        (g * 255.0) as u8,
                        (b * 255.0) as u8,
                        255,
                    ));

                    let _rect_obj = objects
                        .create_path_object_rect(
                            rect,
                            stroke_color,
                            Some(PdfPoints::new(*thickness)),
                            fill_color,
                        )
                        .map_err(|e| e.to_string())?;
                }
                AnnotationStyle::Text {
                    text,
                    color,
                    font_size,
                } => {
                    let (_r, _g, _b) = Self::hex_to_rgb(color);
                    let font = doc.fonts_mut().helvetica();
                    let _text_obj = objects
                        .create_text_object(
                            PdfPoints::new(ann.x),
                            PdfPoints::new(ann.y),
                            text,
                            font,
                            PdfPoints::new(*font_size as f32),
                        )
                        .map_err(|e| e.to_string())?;
                }
            }
        }

        let output_path = pdf_path.replace(".pdf", "_annotated.pdf");
        doc.save_to_file(&output_path).map_err(|e| e.to_string())?;
        Ok(output_path)
    }

    pub fn load_annotations(&self, _pdf_path: &str) -> Result<Vec<Annotation>, String> {
        Ok(Vec::new())
    }

    pub fn export_page_as_image(
        &self,
        path: &str,
        page_num: i32,
        scale: f32,
        output_path: &str,
    ) -> Result<(), String> {
        let state = self
            .documents
            .get(path)
            .ok_or_else(|| "Document not found".to_string())?;
        let page = state.doc.pages().get(page_num).map_err(|e| e.to_string())?;

        let render_config = PdfRenderConfig::new()
            .set_target_width((page.width().value * scale) as i32)
            .set_maximum_height((page.height().value * scale) as i32)
            .rotate(PdfPageRenderRotation::None, false)
            .use_lcd_text_rendering(true);

        let bitmap = page
            .render_with_config(&render_config)
            .map_err(|e| e.to_string())?;
        let img = bitmap.as_image().map_err(|e| e.to_string())?;
        img.save(output_path).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn export_pages_as_images(
        &self,
        path: &str,
        pages: &[i32],
        scale: f32,
        output_dir: &str,
    ) -> Result<Vec<String>, String> {
        let state = self
            .documents
            .get(path)
            .ok_or_else(|| "Document not found".to_string())?;
        let mut paths = Vec::new();

        for &page_num in pages {
            let page = state.doc.pages().get(page_num).map_err(|e| e.to_string())?;

            let render_config = PdfRenderConfig::new()
                .set_target_width((page.width().value * scale) as i32)
                .set_maximum_height((page.height().value * scale) as i32)
                .rotate(PdfPageRenderRotation::None, false)
                .use_lcd_text_rendering(true);

            let bitmap = match page.render_with_config(&render_config) {
                Ok(b) => b,
                Err(_) => continue,
            };
            let img = match bitmap.as_image() {
                Ok(i) => i,
                Err(_) => continue,
            };
            let out_path = format!("{}/page_{}.png", output_dir, page_num + 1);
            if img.save(&out_path).is_ok() {
                paths.push(out_path);
            }
        }
        Ok(paths)
    }

    pub fn get_outline(&self, path: &str) -> Vec<Bookmark> {
        if let Some(state) = self.documents.get(path) {
            let mut bookmarks = Vec::new();
            for b in state.doc.bookmarks().iter() {
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
        } else {
            Vec::new()
        }
    }

    pub fn search(&self, doc_id: &str, query: &str) -> Result<Vec<SearchResultItem>, String> {
        let state = self
            .documents
            .get(doc_id)
            .ok_or_else(|| "Document not found or closed".to_string())?;

        let doc = &state.doc;
        let mut results = Vec::new();

        for (page_idx, page) in doc.pages().iter().enumerate() {
            if let Ok(text) = page.text() {
                let search_options = PdfSearchOptions::new();
                if let Ok(searcher) = text.search(query, &search_options) {
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
                            let x = rect.left().value;
                            let y = rect.bottom().value;
                            let w = rect.width().value;
                            let h = rect.height().value;

                            results.push((page_idx, text_all, y, x, w, h));
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    fn hex_to_rgb(hex: &str) -> (f32, f32, f32) {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return (0.0, 0.0, 0.0);
        }
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f32 / 255.0;
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f32 / 255.0;
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f32 / 255.0;
        (r, g, b)
    }

    fn detect_content_bbox(data: &[u8], width: u32, height: u32) -> Option<(u32, u32, u32, u32)> {
        let mut min_x = width;
        let mut min_y = height;
        let mut max_x = 0;
        let mut max_y = 0;
        let mut found = false;

        let is_bg = |p: &[u8]| {
            let r = p[0];
            let g = p[1];
            let b = p[2];
            r > 245 && g > 245 && b > 245
        };

        for y in 0..height {
            let mut row_empty = true;
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;
                if !is_bg(&data[idx..idx + 4]) {
                    row_empty = false;
                    if x < min_x {
                        min_x = x;
                    }
                    if x > max_x {
                        max_x = x;
                    }
                    found = true;
                }
            }
            if !row_empty {
                if y < min_y {
                    min_y = y;
                }
                if y > max_y {
                    max_y = y;
                }
            }
        }

        if found {
            let margin = 10;
            Some((
                min_x.saturating_sub(margin),
                min_y.saturating_sub(margin),
                (max_x + margin).min(width.saturating_sub(1)),
                (max_y + margin).min(height.saturating_sub(1)),
            ))
        } else {
            None
        }
    }

    fn apply_filter(mut data: Vec<u8>, _width: u32, _height: u32, filter: RenderFilter) -> Vec<u8> {
        match filter {
            RenderFilter::Inverted => {
                for i in (0..data.len()).step_by(4) {
                    data[i] = 255 - data[i];
                    data[i + 1] = 255 - data[i + 1];
                    data[i + 2] = 255 - data[i + 2];
                }
            }
            RenderFilter::Eco => {
                for i in (0..data.len()).step_by(4) {
                    let avg = (data[i] as u32 + data[i + 1] as u32 + data[i + 2] as u32) / 3;
                    if avg > 200 {
                        data[i] = 255;
                        data[i + 1] = 255;
                        data[i + 2] = 255;
                    }
                }
            }
            RenderFilter::BlackWhite => {
                for i in (0..data.len()).step_by(4) {
                    let avg = (data[i] as u32 + data[i + 1] as u32 + data[i + 2] as u32) / 3;
                    let val = if avg > 128 { 255 } else { 0 };
                    data[i] = val;
                    data[i + 1] = val;
                    data[i + 2] = val;
                }
            }
            RenderFilter::Lighten => {
                for i in (0..data.len()).step_by(4) {
                    data[i] = data[i].saturating_add(20);
                    data[i + 1] = data[i + 1].saturating_add(20);
                    data[i + 2] = data[i + 2].saturating_add(20);
                }
            }
            RenderFilter::NoShadow => {
                for i in (0..data.len()).step_by(4) {
                    if data[i] > 230 && data[i + 1] > 230 && data[i + 2] > 230 {
                        data[i] = 255;
                        data[i + 1] = 255;
                        data[i + 2] = 255;
                    }
                }
            }
            _ => {}
        }
        data
    }
}

pub fn create_render_cache(cache_size: u64) -> SharedRenderCache {
    Arc::new(Mutex::new(LruCache::new(
        std::num::NonZeroUsize::new(cache_size as usize)
            .unwrap_or(std::num::NonZeroUsize::new(12).unwrap()),
    )))
}

#[derive(Clone, Debug)]
pub struct Bookmark {
    pub title: String,
    pub page_index: u16,
}
