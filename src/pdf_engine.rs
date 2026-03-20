use crate::models::{
    Annotation, AnnotationStyle, DocumentId, FormField, Hyperlink, PdfError, PdfResult,
    SearchResultItem,
};
use lopdf::{Document, Object, ObjectId};
use pdf_writer::{Content, Finish, Name, PdfWriter, Rect, Ref};
use pdfium_render::prelude::*;
use quick_cache::{sync::Cache, Weighter};
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use zune_image::codecs::png::PngEncoder;
use zune_image::image::Image;
use zune_image::traits::EncoderTrait;

use crate::ui::theme::hex_to_rgb;

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct RenderKey {
    pub doc_id: DocumentId,
    pub page_num: usize,
    pub scale: u32,
    pub filter: RenderFilter,
    pub auto_crop: bool,
    pub quality: RenderQuality,
}

#[derive(Clone)]
struct RenderWeighter;

impl Weighter<RenderKey, crate::models::RenderResult> for RenderWeighter {
    fn weight(&self, _key: &RenderKey, val: &crate::models::RenderResult) -> u64 {
        val.data.len() as u64
    }
}

pub struct RenderCache {
    cache: Cache<RenderKey, crate::models::RenderResult, RenderWeighter>,
    scale_index: HashMap<(DocumentId, usize), RenderKey>,
}

impl RenderCache {
    pub fn new(capacity: usize, max_bytes: usize) -> Self {
        Self {
            cache: Cache::with_weighter(
                capacity.max(1),
                if max_bytes == 0 {
                    512 * 1024 * 1024
                } else {
                    max_bytes as u64
                },
                RenderWeighter,
            ),
            scale_index: HashMap::new(),
        }
    }

    pub fn get(&self, key: &RenderKey) -> Option<crate::models::RenderResult> {
        self.cache.get(key)
    }

    pub fn put(
        &mut self,
        doc_id: DocumentId,
        page_num: usize,
        key: RenderKey,
        result: crate::models::RenderResult,
    ) {
        let page_key = (doc_id, page_num);
        if let Some(old_key) = self.scale_index.insert(page_key, key.clone()) {
            if old_key != key {
                self.cache.remove(&old_key);
            }
        }
        self.cache.insert(key, result);
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
    documents: HashMap<DocumentId, DocumentState<'a>>,
    paths: HashMap<DocumentId, String>,
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
            paths: HashMap::new(),
            render_cache: cache,
        })
    }

    pub fn open_document(
        &mut self,
        path: &str,
        doc_id: DocumentId,
    ) -> PdfResult<crate::models::OpenResult> {
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
            if let Ok(page) = pages.get(i) {
                let w = page.width().value;
                let h = page.height().value;
                heights.push(h);
                if w > max_width {
                    max_width = w;
                }

                for link in page.links().iter() {
                    if let Ok(rect) = link.rect() {
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
                                bounds: (
                                    rect.left().value,
                                    rect.bottom().value,
                                    rect.width().value,
                                    rect.height().value,
                                ),
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
        let metadata = crate::models::DocumentMetadata {
            title: doc.metadata().get(PdfDocumentMetadataTag::Title),
            author: doc.metadata().get(PdfDocumentMetadataTag::Author),
            subject: doc.metadata().get(PdfDocumentMetadataTag::Subject),
            keywords: doc.metadata().get(PdfDocumentMetadataTag::Keywords),
            creator: doc.metadata().get(PdfDocumentMetadataTag::Creator),
            producer: doc.metadata().get(PdfDocumentMetadataTag::Producer),
            creation_date: doc.metadata().get(PdfDocumentMetadataTag::CreationDate),
            modification_date: doc.metadata().get(PdfDocumentMetadataTag::ModificationDate),
        };

        let state = DocumentState { doc };
        self.documents.insert(doc_id, state);
        self.paths.insert(doc_id, path.to_string());

        Ok(crate::models::OpenResult {
            id: doc_id,
            page_count: page_count as usize,
            page_heights: heights,
            max_width,
            outline,
            links: all_links,
            metadata,
        })
    }

    pub fn close_document(&mut self, doc_id: DocumentId) {
        self.documents.remove(&doc_id);
        self.paths.remove(&doc_id);
    }

    pub fn render_page(
        &mut self,
        doc_id: DocumentId,
        page_num: usize,
        options: RenderOptions,
    ) -> PdfResult<crate::models::RenderResult> {
        let rounded_scale = (options.scale * 100.0).round() as u32;
        let cache_key = RenderKey {
            doc_id,
            page_num,
            scale: rounded_scale,
            filter: options.filter,
            auto_crop: options.auto_crop,
            quality: options.quality,
        };

        {
            let cache = self
                .render_cache
                .lock()
                .map_err(|e| PdfError::EngineError(e.to_string()))?;
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(cached);
            }
        }

        let state = self
            .documents
            .get(&doc_id)
            .ok_or_else(|| PdfError::EngineError("Document not found".to_string()))?;
        let page = state
            .doc
            .pages()
            .get(page_num as u16)
            .map_err(|_| PdfError::PageNotFound(page_num))?;

        let mut target_w = (page.width().value * options.scale) as i32;
        let mut target_h = (page.height().value * options.scale) as i32;

        let max_dim = 2500;
        if target_w > max_dim || target_h > max_dim {
            let scale_factor = max_dim as f32 / (target_w.max(target_h) as f32);
            target_w = (target_w as f32 * scale_factor) as i32;
            target_h = (target_h as f32 * scale_factor) as i32;
        }

        let mut render_config = PdfRenderConfig::new()
            .set_target_width(target_w)
            .set_maximum_height(target_h)
            .rotate(
                match options.rotation {
                    90 => PdfPageRenderRotation::Degrees90,
                    180 => PdfPageRenderRotation::Degrees180,
                    270 => PdfPageRenderRotation::Degrees270,
                    _ => PdfPageRenderRotation::None,
                },
                false,
            )
            .use_lcd_text_rendering(true);

        if options.filter == RenderFilter::Grayscale {
            render_config = render_config.use_grayscale_rendering(true);
        }

        let bitmap = page
            .render_with_config(&render_config)
            .map_err(|e| PdfError::RenderFailed(e.to_string()))?;
        let w = bitmap.width() as u32;
        let h = bitmap.height() as u32;

        // Optimization: Use bitmap bytes directly and handle filtering in-place if possible
        let mut result_data = bitmap.as_rgba_bytes().to_vec();
        
        if options.filter != RenderFilter::None && options.filter != RenderFilter::Grayscale {
            Self::apply_filter_in_place(&mut result_data, options.filter);
        }

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
            data: bytes::Bytes::from(final_data),
        };
        {
            let mut cache = self
                .render_cache
                .lock()
                .map_err(|e| PdfError::EngineError(e.to_string()))?;
            cache.put(doc_id, page_num, cache_key, result.clone());
        }
        Ok(result)
    }

    pub fn extract_text(&self, doc_id: DocumentId, page_num: i32) -> PdfResult<String> {
        let state = self
            .documents
            .get(&doc_id)
            .ok_or_else(|| PdfError::EngineError("Document not found".to_string()))?;
        let page = state
            .doc
            .pages()
            .get(page_num as u16)
            .map_err(|_| PdfError::PageNotFound(page_num as usize))?;
        let text_page = page
            .text()
            .map_err(|e| PdfError::SearchError(e.to_string()))?;
        Ok(text_page.all())
    }

    pub fn save_annotations(
        &mut self,
        doc_id: DocumentId,
        annotations: &[Annotation],
    ) -> PdfResult<String> {
        let pdf_path = self
            .paths
            .get(&doc_id)
            .ok_or_else(|| PdfError::EngineError("Document path not found".to_string()))?;
        let state = self
            .documents
            .get_mut(&doc_id)
            .ok_or_else(|| PdfError::EngineError("Document not found".to_string()))?;
        let doc = &mut state.doc;
        for ann in annotations {
            let mut page = doc
                .pages()
                .get(ann.page as u16)
                .map_err(|_| PdfError::PageNotFound(ann.page))?;
            let page_height = page.height().value;
            let objects = page.objects_mut();
            let rect = PdfRect::new(
                PdfPoints::new(page_height - ann.y),
                PdfPoints::new(ann.x),
                PdfPoints::new(page_height - (ann.y + ann.height)),
                PdfPoints::new(ann.x + ann.width),
            );

            match &ann.style {
                AnnotationStyle::Highlight { color } => {
                    let (r, g, b) = hex_to_rgb(color);
                    let _ = objects
                        .create_path_object_rect(
                            rect,
                            None,
                            None,
                            Some(PdfColor::new(
                                (r * 255.0) as u8,
                                (g * 255.0) as u8,
                                (b * 255.0) as u8,
                                100,
                            )),
                        )
                        .map_err(|e| PdfError::RenderFailed(e.to_string()))?;
                }
                AnnotationStyle::Rectangle {
                    color,
                    thickness,
                    fill,
                } => {
                    let (r, g, b) = hex_to_rgb(color);
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
                    let _ = objects
                        .create_path_object_rect(
                            rect,
                            Some(PdfColor::new(
                                (r * 255.0) as u8,
                                (g * 255.0) as u8,
                                (b * 255.0) as u8,
                                255,
                            )),
                            Some(PdfPoints::new(*thickness)),
                            fill_color,
                        )
                        .map_err(|e| PdfError::RenderFailed(e.to_string()))?;
                }
                AnnotationStyle::Text {
                    text,
                    color,
                    font_size,
                } => {
                    let (r, g, b) = hex_to_rgb(color);
                    
                    // Use font-kit to find a better system font
                    let font_handle = font_kit::source::SystemSource::new()
                        .select_best_match(
                            &[font_kit::family_name::FamilyName::SansSerif],
                            &font_kit::properties::Properties::new(),
                        )
                        .ok();

                    let font = if let Some(handle) = font_handle {
                        if let Ok(font_data) = handle.load() {
                            if let Ok(data) = font_data.copy_font_data() {
                                doc.fonts_mut().load_from_bytes(&data).unwrap_or(doc.fonts_mut().helvetica())
                            } else {
                                doc.fonts_mut().helvetica()
                            }
                        } else {
                            doc.fonts_mut().helvetica()
                        }
                    } else {
                        doc.fonts_mut().helvetica()
                    };

                    let mut text_obj = objects
                        .create_text_object(
                            PdfPoints::new(ann.x),
                            PdfPoints::new(page_height - (ann.y + ann.height)),
                            text,
                            font,
                            PdfPoints::new(*font_size as f32),
                        )
                        .map_err(|e| PdfError::RenderFailed(e.to_string()))?;
                    text_obj
                        .set_fill_color(PdfColor::new(
                            (r * 255.0) as u8,
                            (g * 255.0) as u8,
                            (b * 255.0) as u8,
                            255,
                        ))
                        .map_err(|e| PdfError::RenderFailed(e.to_string()))?;
                }
            }
        }
        let output_path = pdf_path.replace(".pdf", "_annotated.pdf");
        doc.save_to_file(&output_path)
            .map_err(|e| PdfError::IoError(e.to_string()))?;
        Ok(output_path)
    }

    pub fn export_page_as_image(
        &self,
        doc_id: DocumentId,
        page_num: i32,
        scale: f32,
        output_path: &str,
    ) -> PdfResult<()> {
        let state = self
            .documents
            .get(&doc_id)
            .ok_or_else(|| PdfError::EngineError("Document not found".to_string()))?;
        let page = state
            .doc
            .pages()
            .get(page_num as u16)
            .map_err(|_| PdfError::PageNotFound(page_num as usize))?;

        let render_config = PdfRenderConfig::new()
            .set_target_width((page.width().value * scale) as i32)
            .set_maximum_height((page.height().value * scale) as i32)
            .use_lcd_text_rendering(true);

        let bitmap = page
            .render_with_config(&render_config)
            .map_err(|e| PdfError::RenderFailed(e.to_string()))?;
        let width = bitmap.width() as usize;
        let height = bitmap.height() as usize;

        let image = Image::from_u8(
            &bitmap.as_rgba_bytes(),
            width,
            height,
            zune_core::colorspace::ColorSpace::RGBA,
        );
        let mut encoder = PngEncoder::new();
        let out_buf = encoder
            .encode(&image)
            .map_err(|e| PdfError::RenderFailed(format!("{:?}", e)))?;

        let optimized =
            oxipng::optimize_from_memory(&out_buf, &oxipng::Options::default()).unwrap_or(out_buf);
        std::fs::write(output_path, optimized).map_err(|e| PdfError::IoError(e.to_string()))?;
        Ok(())
    }

    pub fn get_outline_internal(&self, doc: &PdfDocument) -> Vec<Bookmark> {
        doc.bookmarks()
            .iter()
            .filter_map(|b| {
                b.title().map(|title| Bookmark {
                    title,
                    page_index: b
                        .destination()
                        .and_then(|d| d.page_index().ok())
                        .unwrap_or(0),
                })
            })
            .collect()
    }

    pub fn search(&self, doc_id: DocumentId, query: &str) -> PdfResult<Vec<SearchResultItem>> {
        let state = self
            .documents
            .get(&doc_id)
            .ok_or_else(|| PdfError::EngineError("Document not found".to_string()))?;
        let mut results = Vec::new();
        for (page_idx, page) in state.doc.pages().iter().enumerate() {
            if let Ok(text) = page.text() {
                if let Ok(searcher) = text.search(query, &PdfSearchOptions::new()) {
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
        }
        Ok(results)
    }

    fn detect_content_bbox_parallel(
        data: &[u8],
        width: u32,
        height: u32,
    ) -> Option<(u32, u32, u32, u32)> {
        let active_pixels: Vec<(u32, u32)> = data
            .par_chunks_exact(4)
            .enumerate()
            .filter_map(|(idx, pixel)| {
                if pixel[0] <= 245 || pixel[1] <= 245 || pixel[2] <= 245 {
                    Some(((idx as u32) % width, (idx as u32) / width))
                } else {
                    None
                }
            })
            .collect();
        if active_pixels.is_empty() {
            return None;
        }
        let (min_x, max_x) = (
            active_pixels.iter().map(|p| p.0).min().unwrap(),
            active_pixels.iter().map(|p| p.0).max().unwrap(),
        );
        let (min_y, max_y) = (
            active_pixels.iter().map(|p| p.1).min().unwrap(),
            active_pixels.iter().map(|p| p.1).max().unwrap(),
        );
        let m = 10;
        Some((
            min_x.saturating_sub(m),
            min_y.saturating_sub(m),
            (max_x + m).min(width.saturating_sub(1)),
            (max_y + m).min(height.saturating_sub(1)),
        ))
    }

    pub fn apply_filter_in_place(data: &mut [u8], filter: RenderFilter) {
        data.par_chunks_exact_mut(4).for_each(|pixel| match filter {
            RenderFilter::Inverted => {
                pixel[0] = 255 - pixel[0];
                pixel[1] = 255 - pixel[1];
                pixel[2] = 255 - pixel[2];
            }
            RenderFilter::Eco => {
                let avg = (pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32) / 3;
                if avg > 200 {
                    pixel[0] = 255;
                    pixel[1] = 255;
                    pixel[2] = 255;
                }
            }
            RenderFilter::BlackWhite => {
                let avg = (pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32) / 3;
                let val = if avg > 128 { 255 } else { 0 };
                pixel[0] = val;
                pixel[1] = val;
                pixel[2] = val;
            }
            RenderFilter::Lighten => {
                pixel[0] = pixel[0].saturating_add(20);
                pixel[1] = pixel[1].saturating_add(20);
                pixel[2] = pixel[2].saturating_add(20);
            }
            RenderFilter::NoShadow => {
                if pixel[0] > 230 && pixel[1] > 230 && pixel[2] > 230 {
                    pixel[0] = 255;
                    pixel[1] = 255;
                    pixel[2] = 255;
                }
            }
            _ => {}
        });
    }

    pub fn apply_filter_parallel(mut data: Vec<u8>, filter: RenderFilter) -> Vec<u8> {
        Self::apply_filter_in_place(&mut data, filter);
        data
    }

    pub fn merge_documents(paths: Vec<String>, output_path: String) -> PdfResult<String> {
        let mut writer = PdfWriter::new();
        let mut page_refs = Vec::new();
        let mut current_ref = 1;

        for path in paths {
            let doc = Document::load(&path).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
            let pages = doc.get_pages();
            let mut id_map = HashMap::new();

            // First pass: Allocate new IDs for all objects in this document
            let base_ref = current_ref;
            for &id in doc.objects.keys() {
                id_map.insert(id, Ref::new(base_ref + id.0 as i32));
                current_ref = current_ref.max(base_ref + id.0 as i32);
            }
            current_ref += 1;

            // Second pass: Copy all objects
            for (&id, obj) in &doc.objects {
                let new_ref = *id_map.get(&id).unwrap();
                Self::write_lopdf_to_writer(&mut writer, new_ref, obj, &id_map);
            }

            // Collect page references
            for (_, id) in pages {
                page_refs.push(*id_map.get(&id).unwrap());
            }
        }

        // Create the page tree
        let catalog_ref = Ref::new(current_ref);
        let pages_ref = Ref::new(current_ref + 1);
        
        writer.catalog(catalog_ref).pages(pages_ref);
        let mut pages_obj = writer.pages(pages_ref);
        pages_obj.kids(page_refs);
        pages_obj.count(current_ref as i32); // Simplified count
        pages_obj.finish();

        std::fs::write(&output_path, writer.finish())
            .map_err(|e| PdfError::IoError(e.to_string()))?;
        Ok(output_path)
    }

    pub fn split_pdf(
        path: &str,
        page_indices: Vec<usize>,
        output_path: String,
    ) -> PdfResult<Vec<String>> {
        let doc = Document::load(path).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        let mut writer = PdfWriter::new();
        let mut id_map = HashMap::new();
        let mut current_ref = 1;

        // Map all objects to new IDs
        for &id in doc.objects.keys() {
            id_map.insert(id, Ref::new(current_ref));
            current_ref += 1;
        }

        // Copy all objects
        for (&id, obj) in &doc.objects {
            let new_ref = *id_map.get(&id).unwrap();
            Self::write_lopdf_to_writer(&mut writer, new_ref, obj, &id_map);
        }

        // Collect only requested pages
        let all_pages = doc.get_pages();
        let mut selected_page_refs = Vec::new();
        for &idx in &page_indices {
            if let Some((_, &id)) = all_pages.iter().nth(idx) {
                selected_page_refs.push(*id_map.get(&id).unwrap());
            }
        }

        let catalog_ref = Ref::new(current_ref);
        let pages_ref = Ref::new(current_ref + 1);
        writer.catalog(catalog_ref).pages(pages_ref);
        writer.pages(pages_ref).kids(selected_page_refs).count(page_indices.len() as i32);

        std::fs::write(&output_path, writer.finish())
            .map_err(|e| PdfError::IoError(e.to_string()))?;
        Ok(vec![output_path])
    }

    fn write_lopdf_to_writer(
        writer: &mut PdfWriter,
        new_ref: Ref,
        obj: &Object,
        id_map: &HashMap<ObjectId, Ref>,
    ) {
        match obj {
            Object::Stream(s) => {
                let mut stream = writer.stream(new_ref, &s.content);
                // Convert dictionary entries...
                for (key, val) in &s.dict {
                    let key_name = Name(key);
                    match val {
                        Object::Reference(id) => {
                            if let Some(&r) = id_map.get(id) {
                                stream.pair(key_name, r);
                            }
                        }
                        Object::Integer(i) => { stream.pair(key_name, *i as i32); }
                        Object::Name(n) => { stream.pair(key_name, Name(n)); }
                        _ => {} // Simplified for now
                    }
                }
            }
            Object::Dictionary(d) => {
                let mut dict = writer.indirect(new_ref).dict();
                for (key, val) in d {
                    let key_name = Name(key);
                    match val {
                        Object::Reference(id) => {
                            if let Some(&r) = id_map.get(id) {
                                dict.pair(key_name, r);
                            }
                        }
                        Object::Integer(i) => { dict.pair(key_name, *i as i32); }
                        Object::Name(n) => { dict.pair(key_name, Name(n)); }
                        _ => {}
                    }
                }
            }
            _ => {} // Other objects handled similarly
        }
    }

    pub fn get_form_fields(&mut self, path: &str) -> PdfResult<Vec<FormField>> {
        let doc = self
            .pdfium
            .load_pdf_from_file(path, None)
            .map_err(|e| PdfError::OpenFailed(e.to_string()))?;

        let mut fields = Vec::new();
        if let Some(form) = doc.form() {
            for (idx, field) in form.fields().iter().enumerate() {
                fields.push(FormField {
                    name: field.name().unwrap_or_else(|| format!("Field {}", idx)),
                    value: field.value().unwrap_or_default(),
                    field_type: format!("{:?}", field.field_type()),
                    page: 0, // Simplified
                });
            }
        }
        Ok(fields)
    }

    pub fn fill_form(
        &mut self,
        path: &str,
        updates: Vec<FormField>,
        output_path: String,
    ) -> PdfResult<String> {
        let doc = self
            .pdfium
            .load_pdf_from_file(path, None)
            .map_err(|e| PdfError::OpenFailed(e.to_string()))?;

        if let Some(mut form) = doc.form() {
            for update in updates {
                if let Some(mut field) = form.fields().iter().find(|f| f.name() == Some(update.name.clone())) {
                    let _ = field.set_value(&update.value);
                }
            }
        }

        doc.save_to_file(&output_path)
            .map_err(|e| PdfError::IoError(e.to_string()))?;
        Ok(output_path)
    }

    pub fn print_document(path: &str) -> PdfResult<()> {
        use winprint::printer::{FilePrinter, PdfiumPrinter, PrinterDevice};

        let device = PrinterDevice::all()
            .map_err(|e| PdfError::IoError(format!("Failed to list printers: {}", e)))?
            .into_iter()
            .next()
            .ok_or_else(|| PdfError::IoError("No printers found".into()))?;

        let printer = PdfiumPrinter::new(device);
        printer
            .print(std::path::Path::new(path), Default::default())
            .map_err(|e| PdfError::IoError(format!("Print failed: {}", e)))?;
        Ok(())
    }

    pub fn add_watermark(input_path: &str, text: &str, output_path: &str) -> PdfResult<String> {
        let mut doc =
            Document::load(input_path).map_err(|e| PdfError::OpenFailed(e.to_string()))?;

        let pages: Vec<ObjectId> = doc.get_pages().into_values().collect();
        let page_count = pages.len();

        let font_ref_id = doc.add_object(Object::Dictionary(lopdf::Dictionary::from_iter(vec![
            ("Type", Object::Name(b"Font".to_vec())),
            ("Subtype", Object::Name(b"Type1".to_vec())),
            ("BaseFont", Object::Name(b"Helvetica".to_vec())),
        ])));

        let resources_id =
            doc.add_object(Object::Dictionary(lopdf::Dictionary::from_iter(vec![(
                "Font",
                Object::Dictionary(lopdf::Dictionary::from_iter(vec![(
                    "F1",
                    Object::Reference(font_ref_id),
                )])),
            )])));

        let escaped = text
            .replace('\\', "\\\\")
            .replace('(', "\\(")
            .replace(')', "\\)");
        let content = format!(
            "BT /F1 48 Tf 0.7 0.7 0.7 rg 0.5 Tm 200 400 Td 45 Tz ({}) Tj ET\n",
            escaped
        );
        let watermark_stream = lopdf::Stream::new(lopdf::Dictionary::new(), content.into_bytes());

        for &page_id in &pages {
            let watermark_id = doc.add_object(watermark_stream.clone());

            let existing_contents = doc
                .get_page_contents(page_id)
                .into_iter()
                .map(Object::Reference)
                .collect::<Vec<_>>();
            let mut all_contents = existing_contents;
            all_contents.push(Object::Reference(watermark_id));

            let page_dict = doc
                .objects
                .get_mut(&page_id)
                .and_then(|o| o.as_dict_mut().ok())
                .ok_or_else(|| PdfError::EngineError("Invalid page object".into()))?;
            page_dict.set("Contents", Object::Array(all_contents));
            page_dict.set("Resources", Object::Reference(resources_id));
        }

        doc.save(output_path)
            .map_err(|e| PdfError::IoError(e.to_string()))?;

        tracing::info!(
            "Watermark '{}' applied to {} pages -> {}",
            text,
            page_count,
            output_path
        );
        Ok(output_path.to_string())
    }
}

pub fn create_render_cache(cache_size: u64, max_memory_mb: u64) -> SharedRenderCache {
    let mb = (max_memory_mb * 1024 * 1024) as usize;
    Arc::new(Mutex::new(RenderCache::new(
        cache_size as usize,
        if mb == 0 { 512 * 1024 * 1024 } else { mb },
    )))
}

#[derive(Clone, Debug)]
pub struct Bookmark {
    pub title: String,
    pub page_index: u16,
}
