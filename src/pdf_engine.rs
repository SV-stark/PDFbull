use crate::models::{
    Annotation, AnnotationStyle, DocumentId, EngineErrorKind, FormField, FormFieldVariant,
    Hyperlink, PdfError, PdfResult, SearchResultItem,
};
use lopdf::{Document, Object, ObjectId};
use pdfium_render::prelude::*;
use quick_cache::{Weighter, sync::Cache};
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use zune_image::codecs::png::PngEncoder;
use zune_image::image::Image;
use zune_image::traits::EncoderTrait;

use crate::ui::theme::hex_to_rgb;

const MAX_RENDER_DIM: i32 = 2500;
const WHITE_THRESHOLD: u8 = 245;
const BBOX_MARGIN: u32 = 10;
const NO_SHADOW_THRESHOLD: u8 = 230;

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
        }
    }

    pub fn get(&self, key: &RenderKey) -> Option<crate::models::RenderResult> {
        self.cache.get(key)
    }

    pub fn put(&self, key: RenderKey, result: crate::models::RenderResult) {
        self.cache.insert(key, result);
    }
}

pub type SharedRenderCache = Arc<RenderCache>;

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize, Hash, Eq)]
pub enum RenderQuality {
    Low,
    Medium,
    High,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize, Hash, Eq)]
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
    documents: HashMap<DocumentId, PdfDocument<'a>>,
    paths: HashMap<DocumentId, String>,
    render_cache: SharedRenderCache,
}

// DocumentState wrapper removed as it was a single-field struct.

impl<'a> DocumentStore<'a> {
    pub fn new(pdfium: &'a Pdfium, cache: SharedRenderCache) -> Self {
        Self {
            pdfium,
            documents: HashMap::new(),
            paths: HashMap::new(),
            render_cache: cache,
        }
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
        let signatures = self.extract_signatures_internal(&doc);
        let pdf_metadata = doc.metadata();
        let metadata = crate::models::DocumentMetadata {
            title: pdf_metadata
                .get(PdfDocumentMetadataTagType::Title)
                .map(|t| t.value().to_string()),
            author: pdf_metadata
                .get(PdfDocumentMetadataTagType::Author)
                .map(|t| t.value().to_string()),
            subject: pdf_metadata
                .get(PdfDocumentMetadataTagType::Subject)
                .map(|t| t.value().to_string()),
            keywords: pdf_metadata
                .get(PdfDocumentMetadataTagType::Keywords)
                .map(|t| t.value().to_string()),
            creator: pdf_metadata
                .get(PdfDocumentMetadataTagType::Creator)
                .map(|t| t.value().to_string()),
            producer: pdf_metadata
                .get(PdfDocumentMetadataTagType::Producer)
                .map(|t| t.value().to_string()),
            creation_date: pdf_metadata
                .get(PdfDocumentMetadataTagType::CreationDate)
                .map(|t| t.value().to_string()),
            modification_date: pdf_metadata
                .get(PdfDocumentMetadataTagType::ModificationDate)
                .map(|t| t.value().to_string()),
        };

        self.documents.insert(doc_id, doc);
        self.paths.insert(doc_id, path.to_string());

        Ok(crate::models::OpenResult {
            id: doc_id,
            page_count: page_count as usize,
            page_heights: heights,
            max_width,
            outline,
            links: all_links,
            metadata,
            signatures,
        })
    }

    pub fn close_document(&mut self, doc_id: DocumentId) {
        self.documents.remove(&doc_id);
        self.paths.remove(&doc_id);
    }

    fn render_page_internal(
        &self,
        doc_id: DocumentId,
        page_num: usize,
        options: RenderOptions,
        is_thumbnail: bool,
    ) -> PdfResult<crate::models::RenderResult> {
        let rounded_scale = (options.scale * 100.0).round() as u32;
        let cache_key = RenderKey {
            doc_id,
            page_num,
            scale: rounded_scale,
            filter: if is_thumbnail {
                RenderFilter::None
            } else {
                options.filter
            },
            auto_crop: if is_thumbnail {
                false
            } else {
                options.auto_crop
            },
            quality: if is_thumbnail {
                RenderQuality::Low
            } else {
                options.quality
            },
        };

        if let Some(cached) = self.render_cache.get(&cache_key) {
            return Ok(cached);
        }

        let doc = self
            .documents
            .get(&doc_id)
            .ok_or(PdfError::EngineError(EngineErrorKind::DocumentNotFound))?;
        let page = doc
            .pages()
            .get(page_num as u16)
            .map_err(|_| PdfError::PageNotFound(page_num))?;

        let mut target_w = (page.width().value * options.scale) as i32;
        let mut target_h = (page.height().value * options.scale) as i32;

        if !is_thumbnail && (target_w > MAX_RENDER_DIM || target_h > MAX_RENDER_DIM) {
            let scale_factor = MAX_RENDER_DIM as f32 / (target_w.max(target_h) as f32);
            target_w = (target_w as f32 * scale_factor) as i32;
            target_h = (target_h as f32 * scale_factor) as i32;
        }

        let mut render_config = PdfRenderConfig::new()
            .set_target_width(target_w)
            .set_maximum_height(target_h)
            .use_lcd_text_rendering(!is_thumbnail)
            .rotate(
                match options.rotation {
                    90 => PdfPageRenderRotation::Degrees90,
                    180 => PdfPageRenderRotation::Degrees180,
                    270 => PdfPageRenderRotation::Degrees270,
                    _ => PdfPageRenderRotation::None,
                },
                false,
            );

        if is_thumbnail {
            render_config = render_config.clear_before_rendering(true);
        } else if options.filter == RenderFilter::Grayscale {
            render_config = render_config.use_grayscale_rendering(true);
        }

        let bitmap = page
            .render_with_config(&render_config)
            .map_err(|e| PdfError::RenderFailed(e.to_string()))?;
        let w = bitmap.width() as u32;
        let h = bitmap.height() as u32;

        let (final_w, final_h, final_data) = if !is_thumbnail && options.auto_crop {
            let mut result_data = bitmap.as_rgba_bytes().to_vec();
            if options.filter != RenderFilter::None && options.filter != RenderFilter::Grayscale {
                Self::apply_filter(&mut result_data, options.filter);
            }

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
        } else if !is_thumbnail
            && options.filter != RenderFilter::None
            && options.filter != RenderFilter::Grayscale
        {
            let mut result_data = bitmap.as_rgba_bytes().to_vec();
            Self::apply_filter(&mut result_data, options.filter);
            (w, h, result_data)
        } else {
            (w, h, bitmap.as_rgba_bytes().to_vec())
        };

        let mut text_items = Vec::new();
        if !is_thumbnail && let Ok(text_page) = page.text() {
            let mut current_word = String::new();
            let mut word_rect: Option<PdfRect> = None;

            for char_obj in text_page.chars().iter() {
                let Some(c) = char_obj.unicode_string() else {
                    continue;
                };
                let Ok(bounds) = char_obj.loose_bounds() else {
                    continue;
                };

                if c.trim().is_empty() {
                    if !current_word.is_empty() {
                        if let Some(rect) = word_rect {
                            text_items.push(crate::models::TextItem {
                                text: current_word.clone(),
                                x: rect.left().value,
                                y: page.height().value - rect.top().value,
                                width: (rect.right().value - rect.left().value).abs(),
                                height: (rect.top().value - rect.bottom().value).abs(),
                            });
                        }
                        current_word.clear();
                        word_rect = None;
                    }
                } else {
                    current_word.push_str(&c);
                    if let Some(rect) = word_rect {
                        word_rect = Some(PdfRect::new(
                            rect.bottom().min(bounds.bottom()),
                            rect.left().min(bounds.left()),
                            rect.top().max(bounds.top()),
                            rect.right().max(bounds.right()),
                        ));
                    } else {
                        word_rect = Some(bounds);
                    }
                }
            }
            if !current_word.is_empty()
                && let Some(rect) = word_rect
            {
                text_items.push(crate::models::TextItem {
                    text: current_word,
                    x: rect.left().value,
                    y: page.height().value - rect.top().value,
                    width: (rect.right().value - rect.left().value).abs(),
                    height: (rect.top().value - rect.bottom().value).abs(),
                });
            }
        }

        let result = crate::models::RenderResult {
            width: final_w,
            height: final_h,
            data: bytes::Bytes::from(final_data),
            text_items,
        };

        self.render_cache.put(cache_key, result.clone());
        Ok(result)
    }

    pub fn render_page(
        &mut self,
        doc_id: DocumentId,
        page_num: usize,
        options: RenderOptions,
    ) -> PdfResult<crate::models::RenderResult> {
        self.render_page_internal(doc_id, page_num, options, false)
    }

    pub fn render_thumbnail(
        &mut self,
        doc_id: DocumentId,
        page_num: usize,
        options: RenderOptions,
    ) -> PdfResult<crate::models::RenderResult> {
        self.render_page_internal(doc_id, page_num, options, true)
    }

    pub fn extract_text(&self, doc_id: DocumentId, page_num: i32) -> PdfResult<String> {
        let doc = self
            .documents
            .get(&doc_id)
            .ok_or(PdfError::EngineError(EngineErrorKind::DocumentNotFound))?;
        let page = doc
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
        output_path: Option<String>,
    ) -> PdfResult<String> {
        let pdf_path = self
            .paths
            .get(&doc_id)
            .ok_or(PdfError::EngineError(EngineErrorKind::DocumentPathNotFound))?;
        let doc = self
            .documents
            .get_mut(&doc_id)
            .ok_or(PdfError::EngineError(EngineErrorKind::DocumentNotFound))?;

        // Move font lookup outside the loop
        let font_handle = font_kit::source::SystemSource::new()
            .select_best_match(
                &[font_kit::family_name::FamilyName::SansSerif],
                &font_kit::properties::Properties::new(),
            )
            .ok();

        let annotation_font = if let Some(handle) = font_handle {
            if let Ok(font_data) = handle.load() {
                if let Some(data) = font_data.copy_font_data() {
                    let _ = doc.fonts_mut().load_type1_from_bytes(&data, false);
                    doc.fonts_mut().helvetica()
                } else {
                    doc.fonts_mut().helvetica()
                }
            } else {
                doc.fonts_mut().helvetica()
            }
        } else {
            doc.fonts_mut().helvetica()
        };

        for ann in annotations {
            let mut page = doc
                .pages()
                .get(ann.page as u16)
                .map_err(|_| PdfError::PageNotFound(ann.page))?;
            let page_height = page.height().value;
            let objects = page.objects_mut();

            // PDF coordinates start from bottom-left.
            // UI coordinates start from top-left.
            let rect = PdfRect::new(
                PdfPoints::new(page_height - (ann.y + ann.height)),
                PdfPoints::new(ann.x),
                PdfPoints::new(page_height - ann.y),
                PdfPoints::new(ann.x + ann.width),
            );

            match &ann.style {
                AnnotationStyle::Highlight { color } => {
                    let (r, g, b) = hex_to_rgb(color);
                    objects
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
                    objects
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
                    let font = annotation_font;
                    let (r, g, b) = hex_to_rgb(color);

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
                AnnotationStyle::Redact { color } => {
                    let (r, g, b) = hex_to_rgb(color);
                    // For now, we simulate redaction by adding a solid rectangle.
                    // A pro app would use the actual redaction annotations and call apply_redactions().
                    objects
                        .create_path_object_rect(
                            rect,
                            None,
                            None,
                            Some(PdfColor::new(
                                (r * 255.0) as u8,
                                (g * 255.0) as u8,
                                (b * 255.0) as u8,
                                255, // Solid
                            )),
                        )
                        .map_err(|e| PdfError::RenderFailed(e.to_string()))?;
                }
            }
        }
        let pdf_path_buf = std::path::Path::new(pdf_path);
        let final_path = output_path.unwrap_or_else(|| {
            let mut p = pdf_path_buf.to_path_buf();
            let stem = p
                .file_stem()
                .map(|s| s.to_string_lossy())
                .unwrap_or_default();
            p.set_file_name(format!("{stem}_annotated.pdf"));
            p.to_string_lossy().to_string()
        });
        doc.save_to_file(&final_path)
            .map_err(|e| PdfError::IoError(e.to_string()))?;
        Ok(final_path)
    }

    pub fn export_page_as_image(
        &self,
        doc_id: DocumentId,
        page_num: i32,
        scale: f32,
    ) -> PdfResult<Vec<u8>> {
        let doc = self
            .documents
            .get(&doc_id)
            .ok_or(PdfError::EngineError(EngineErrorKind::DocumentNotFound))?;
        let page = doc
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
            .map_err(|e| PdfError::RenderFailed(format!("{e:?}")))?;

        Ok(out_buf)
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
        let doc = self
            .documents
            .get(&doc_id)
            .ok_or(PdfError::EngineError(EngineErrorKind::DocumentNotFound))?;
        let mut results = Vec::new();
        for (page_idx, page) in doc.pages().iter().enumerate() {
            if let Ok(text) = page.text()
                && let Ok(searcher) = text.search(query, &PdfSearchOptions::new())
            {
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

    fn detect_content_bbox_parallel(
        data: &[u8],
        width: u32,
        height: u32,
    ) -> Option<(u32, u32, u32, u32)> {
        let bbox = data
            .par_chunks_exact(4)
            .enumerate()
            .fold(
                || None::<(u32, u32, u32, u32)>,
                |acc, (idx, pixel)| {
                    if pixel[0] <= WHITE_THRESHOLD
                        || pixel[1] <= WHITE_THRESHOLD
                        || pixel[2] <= WHITE_THRESHOLD
                    {
                        let x = (idx as u32) % width;
                        let y = (idx as u32) / width;
                        if let Some((min_x, min_y, max_x, max_y)) = acc {
                            Some((min_x.min(x), min_y.min(y), max_x.max(x), max_y.max(y)))
                        } else {
                            Some((x, y, x, y))
                        }
                    } else {
                        acc
                    }
                },
            )
            .reduce(
                || None,
                |a, b| match (a, b) {
                    (Some(a), Some(b)) => {
                        Some((a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3)))
                    }
                    (Some(a), None) => Some(a),
                    (None, Some(b)) => Some(b),
                    (None, None) => None,
                },
            );

        bbox.map(|(min_x, min_y, max_x, max_y)| {
            (
                min_x.saturating_sub(BBOX_MARGIN),
                min_y.saturating_sub(BBOX_MARGIN),
                (max_x + BBOX_MARGIN).min(width.saturating_sub(1)),
                (max_y + BBOX_MARGIN).min(height.saturating_sub(1)),
            )
        })
    }

    pub fn apply_filter(data: &mut [u8], filter: RenderFilter) {
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
            RenderFilter::NoShadow
                if pixel[0] > NO_SHADOW_THRESHOLD
                    && pixel[1] > NO_SHADOW_THRESHOLD
                    && pixel[2] > NO_SHADOW_THRESHOLD =>
            {
                pixel[0] = 255;
                pixel[1] = 255;
                pixel[2] = 255;
            }
            _ => {}
        });
    }

    // apply_filter_parallel removed as it was just a misleading wrapper.

    pub fn merge_documents(&self, paths: Vec<String>, output_path: String) -> PdfResult<String> {
        let mut dest = self
            .pdfium
            .create_new_pdf()
            .map_err(|e| PdfError::EngineError(e.to_string().into()))?;

        for path in paths {
            let src = self
                .pdfium
                .load_pdf_from_file(&path, None)
                .map_err(|e| PdfError::OpenFailed(e.to_string()))?;

            let count = src.pages().len();
            if count > 0 {
                let dest_index = dest.pages().len();
                dest.pages_mut()
                    .copy_page_range_from_document(&src, 0..=(count - 1), dest_index)
                    .map_err(|e| PdfError::EngineError(e.to_string().into()))?;
            }
        }

        dest.save_to_file(&output_path)
            .map_err(|e| PdfError::IoError(e.to_string()))?;
        Ok(output_path)
    }

    pub fn split_pdf(
        &self,
        path: &str,
        page_indices: Vec<usize>,
        output_dir: String,
    ) -> PdfResult<Vec<String>> {
        let src = self
            .pdfium
            .load_pdf_from_file(path, None)
            .map_err(|e| PdfError::OpenFailed(e.to_string()))?;

        let mut created_paths = Vec::new();

        for &page_idx in &page_indices {
            let mut dest = self
                .pdfium
                .create_new_pdf()
                .map_err(|e| PdfError::EngineError(e.to_string().into()))?;

            dest.pages_mut()
                .copy_page_range_from_document(&src, (page_idx as u16)..=(page_idx as u16), 0)
                .map_err(|e| PdfError::EngineError(e.to_string().into()))?;

            let filename = std::path::Path::new(path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("document");

            let out_path = format!("{}/{}_page_{}.pdf", output_dir, filename, page_idx + 1);
            dest.save_to_file(&out_path)
                .map_err(|e| PdfError::IoError(e.to_string()))?;
            created_paths.push(out_path);
        }

        Ok(created_paths)
    }

    pub fn get_form_fields(&mut self, path: &str) -> PdfResult<Vec<FormField>> {
        if let Some(doc_id) = self
            .paths
            .iter()
            .find(|(_, p)| *p == path)
            .map(|(id, _)| *id)
        {
            let doc = self.documents.get(&doc_id).unwrap();
            Ok(self.extract_form_fields_internal(doc))
        } else {
            // Load temporarily if not open
            let doc = self
                .pdfium
                .load_pdf_from_file(path, None)
                .map_err(|e| PdfError::OpenFailed(e.to_string()))?;
            Ok(self.extract_form_fields_internal(&doc))
        }
    }

    fn extract_form_fields_internal(&self, doc: &PdfDocument) -> Vec<FormField> {
        let mut fields = Vec::new();
        for (idx, page) in doc.pages().iter().enumerate() {
            for annotation in page.annotations().iter() {
                if let Some(form_field) = annotation.as_form_field() {
                    let name = form_field.name().unwrap_or_default();
                    let variant = match form_field.field_type() {
                        PdfFormFieldType::Text => FormFieldVariant::Text {
                            value: form_field
                                .as_text_field()
                                .and_then(pdfium_render::prelude::PdfFormTextField::value)
                                .unwrap_or_default(),
                        },
                        PdfFormFieldType::Checkbox => FormFieldVariant::Checkbox {
                            is_checked: form_field
                                .as_checkbox_field()
                                .map(|f| f.is_checked().unwrap_or(false))
                                .unwrap_or(false),
                        },
                        PdfFormFieldType::RadioButton => FormFieldVariant::RadioButton {
                            is_selected: form_field
                                .as_radio_button_field()
                                .map(|f| f.is_checked().unwrap_or(false))
                                .unwrap_or(false),
                            group_name: None,
                        },
                        PdfFormFieldType::ComboBox | PdfFormFieldType::ListBox => {
                            FormFieldVariant::ComboBox {
                                options: Vec::new(),
                                selected_index: None,
                            }
                        }
                        _ => FormFieldVariant::Text {
                            value: "".to_string(),
                        },
                    };
                    fields.push(FormField {
                        name,
                        variant,
                        page: idx,
                    });
                }
            }
        }
        fields
    }

    const fn extract_signatures_internal(
        &self,
        _doc: &PdfDocument,
    ) -> Vec<crate::models::SignatureInfo> {
        // Real signature verification requires complex cryptographic logic.
        // For now, we'll return a placeholder to show the UI integration.
        // In a production app, we would use pdfium's signature API or a crate like `openssl`
        Vec::new()
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

        for mut page in doc.pages().iter() {
            let annotations = page.annotations_mut();
            for mut annotation in annotations.iter() {
                if let Some(form_field) = annotation.as_form_field_mut()
                    && let Some(update) = updates
                        .iter()
                        .find(|f| f.name == form_field.name().unwrap_or_default())
                {
                    match &update.variant {
                        FormFieldVariant::Text { value } => {
                            if let Some(text_field) = form_field.as_text_field_mut() {
                                let _ = text_field.set_value(value);
                            }
                        }
                        FormFieldVariant::Checkbox { is_checked } => {
                            if let Some(cb) = form_field.as_checkbox_field_mut() {
                                let _ = cb.set_checked(*is_checked);
                            }
                        }
                        FormFieldVariant::RadioButton { is_selected, .. } => {
                            if *is_selected && let Some(rb) = form_field.as_radio_button_field_mut()
                            {
                                let _ = rb.set_checked();
                            }
                        }
                        FormFieldVariant::ComboBox { .. } => {
                            // TODO: Verify ComboBox/ChoiceField mutation API in pdfium-render
                        }
                    }
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
            .map_err(|e| PdfError::IoError(format!("Failed to list printers: {e}")))?
            .into_iter()
            .next()
            .ok_or_else(|| PdfError::IoError("No printers found".into()))?;

        let printer = PdfiumPrinter::new(device);
        printer
            .print(std::path::Path::new(path), Default::default())
            .map_err(|e| PdfError::IoError(format!("Print failed: {e}")))?;
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
        let content =
            format!("BT /F1 48 Tf 0.7 0.7 0.7 rg 0.5 Tm 200 400 Td 45 Tz ({escaped}) Tj ET\n");
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
    Arc::new(RenderCache::new(
        cache_size as usize,
        if mb == 0 { 512 * 1024 * 1024 } else { mb },
    ))
}

#[derive(Clone, Debug)]
pub struct Bookmark {
    pub title: String,
    pub page_index: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_key_equality() {
        let doc_id = DocumentId(1);
        let key1 = RenderKey {
            doc_id,
            page_num: 0,
            scale: 100,
            filter: RenderFilter::None,
            auto_crop: false,
            quality: RenderQuality::Medium,
        };
        let key2 = RenderKey {
            doc_id,
            page_num: 0,
            scale: 100,
            filter: RenderFilter::None,
            auto_crop: false,
            quality: RenderQuality::Medium,
        };
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_render_key_different_pages() {
        let doc_id = DocumentId(1);
        let key1 = RenderKey {
            doc_id,
            page_num: 0,
            scale: 100,
            filter: RenderFilter::None,
            auto_crop: false,
            quality: RenderQuality::Medium,
        };
        let key2 = RenderKey {
            doc_id,
            page_num: 1,
            scale: 100,
            filter: RenderFilter::None,
            auto_crop: false,
            quality: RenderQuality::Medium,
        };
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_render_key_different_scales() {
        let doc_id = DocumentId(1);
        let key1 = RenderKey {
            doc_id,
            page_num: 0,
            scale: 100,
            filter: RenderFilter::None,
            auto_crop: false,
            quality: RenderQuality::Medium,
        };
        let key2 = RenderKey {
            doc_id,
            page_num: 0,
            scale: 200,
            filter: RenderFilter::None,
            auto_crop: false,
            quality: RenderQuality::Medium,
        };
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_render_key_different_documents() {
        let key1 = RenderKey {
            doc_id: DocumentId(1),
            page_num: 0,
            scale: 100,
            filter: RenderFilter::None,
            auto_crop: false,
            quality: RenderQuality::Medium,
        };
        let key2 = RenderKey {
            doc_id: DocumentId(2),
            page_num: 0,
            scale: 100,
            filter: RenderFilter::None,
            auto_crop: false,
            quality: RenderQuality::Medium,
        };
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_render_key_with_filters() {
        let doc_id = DocumentId(1);
        let key_grayscale = RenderKey {
            doc_id,
            page_num: 0,
            scale: 100,
            filter: RenderFilter::Grayscale,
            auto_crop: false,
            quality: RenderQuality::Medium,
        };
        let key_inverted = RenderKey {
            doc_id,
            page_num: 0,
            scale: 100,
            filter: RenderFilter::Inverted,
            auto_crop: false,
            quality: RenderQuality::Medium,
        };
        assert_ne!(key_grayscale, key_inverted);
    }

    #[test]
    fn test_render_cache_creation() {
        let cache = RenderCache::new(10, 100);
        assert_eq!(
            cache.get(&RenderKey {
                doc_id: DocumentId(1),
                page_num: 0,
                scale: 100,
                filter: RenderFilter::None,
                auto_crop: false,
                quality: RenderQuality::Medium,
            }),
            None
        );
    }

    #[test]
    fn test_render_cache_insert_and_get() {
        let cache = RenderCache::new(10, 1024);
        let key = RenderKey {
            doc_id: DocumentId(1),
            page_num: 0,
            scale: 100,
            filter: RenderFilter::None,
            auto_crop: false,
            quality: RenderQuality::Medium,
        };
        let result = crate::models::RenderResult {
            width: 100,
            height: 100,
            data: bytes::Bytes::from(vec![0u8; 100]),
            text_items: vec![],
        };
        cache.put(key.clone(), result.clone());
        let cached = cache.get(&key);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().width, 100);
    }

    #[test]
    fn test_render_cache_overwrite() {
        let cache = RenderCache::new(10, 1024);
        let key = RenderKey {
            doc_id: DocumentId(1),
            page_num: 0,
            scale: 100,
            filter: RenderFilter::None,
            auto_crop: false,
            quality: RenderQuality::Medium,
        };
        let result1 = crate::models::RenderResult {
            width: 100,
            height: 100,
            data: bytes::Bytes::from(vec![0u8; 100]),
            text_items: vec![],
        };
        let result2 = crate::models::RenderResult {
            width: 200,
            height: 200,
            data: bytes::Bytes::from(vec![0u8; 200]),
            text_items: vec![],
        };
        cache.put(key.clone(), result1);
        cache.put(key.clone(), result2);
        let cached = cache.get(&key);
        assert_eq!(cached.unwrap().width, 200);
    }

    #[test]
    fn test_render_cache_different_keys() {
        let cache = RenderCache::new(10, 1024);
        let key1 = RenderKey {
            doc_id: DocumentId(1),
            page_num: 0,
            scale: 100,
            filter: RenderFilter::None,
            auto_crop: false,
            quality: RenderQuality::Medium,
        };
        let key2 = RenderKey {
            doc_id: DocumentId(1),
            page_num: 1,
            scale: 100,
            filter: RenderFilter::None,
            auto_crop: false,
            quality: RenderQuality::Medium,
        };
        let result1 = crate::models::RenderResult {
            width: 100,
            height: 100,
            data: bytes::Bytes::from(vec![0u8; 100]),
            text_items: vec![],
        };
        let result2 = crate::models::RenderResult {
            width: 200,
            height: 200,
            data: bytes::Bytes::from(vec![0u8; 200]),
            text_items: vec![],
        };
        cache.put(key1.clone(), result1);
        cache.put(key2.clone(), result2);
        assert_eq!(cache.get(&key1).unwrap().width, 100);
        assert_eq!(cache.get(&key2).unwrap().width, 200);
    }

    #[test]
    fn test_apply_filter_inverted() {
        let mut data = vec![100, 150, 200, 255, 50, 75, 100, 255];
        DocumentStore::apply_filter(&mut data, RenderFilter::Inverted);
        assert_eq!(data[0], 155);
        assert_eq!(data[1], 105);
        assert_eq!(data[2], 55);
    }

    #[test]
    fn test_apply_filter_eco_bright() {
        let mut data = vec![250, 250, 250, 255];
        DocumentStore::apply_filter(&mut data, RenderFilter::Eco);
        assert_eq!(data[0], 255);
        assert_eq!(data[1], 255);
        assert_eq!(data[2], 255);
    }

    #[test]
    fn test_apply_filter_eco_dark() {
        let mut data = vec![100, 100, 100, 255];
        DocumentStore::apply_filter(&mut data, RenderFilter::Eco);
        assert_eq!(data[0], 100);
        assert_eq!(data[1], 100);
        assert_eq!(data[2], 100);
    }

    #[test]
    fn test_apply_filter_black_white_high() {
        let mut data = vec![200, 200, 200, 255];
        DocumentStore::apply_filter(&mut data, RenderFilter::BlackWhite);
        assert_eq!(data[0], 255);
        assert_eq!(data[1], 255);
        assert_eq!(data[2], 255);
    }

    #[test]
    fn test_apply_filter_black_white_low() {
        let mut data = vec![50, 50, 50, 255];
        DocumentStore::apply_filter(&mut data, RenderFilter::BlackWhite);
        assert_eq!(data[0], 0);
        assert_eq!(data[1], 0);
        assert_eq!(data[2], 0);
    }

    #[test]
    fn test_apply_filter_lighten() {
        let mut data = vec![100, 100, 100, 255, 230, 230, 230, 255];
        DocumentStore::apply_filter(&mut data, RenderFilter::Lighten);
        assert_eq!(data[0], 120);
        assert_eq!(data[1], 120);
        assert_eq!(data[2], 120);
        assert_eq!(data[4], 250);
    }

    #[test]
    fn test_apply_filter_lighten_saturation() {
        let mut data = vec![245, 245, 245, 255];
        DocumentStore::apply_filter(&mut data, RenderFilter::Lighten);
        assert_eq!(data[0], 255);
        assert_eq!(data[1], 255);
        assert_eq!(data[2], 255);
    }

    #[test]
    fn test_apply_filter_no_shadow_bright() {
        let mut data = vec![235, 235, 235, 255];
        DocumentStore::apply_filter(&mut data, RenderFilter::NoShadow);
        assert_eq!(data[0], 255);
        assert_eq!(data[1], 255);
        assert_eq!(data[2], 255);
    }

    #[test]
    fn test_apply_filter_no_shadow_dark() {
        let mut data = vec![100, 100, 100, 255];
        DocumentStore::apply_filter(&mut data, RenderFilter::NoShadow);
        assert_eq!(data[0], 100);
        assert_eq!(data[1], 100);
        assert_eq!(data[2], 100);
    }

    #[test]
    fn test_apply_filter_grayscale_does_nothing() {
        let mut data = vec![100, 150, 200, 255];
        DocumentStore::apply_filter(&mut data, RenderFilter::Grayscale);
        assert_eq!(data[0], 100);
        assert_eq!(data[1], 150);
        assert_eq!(data[2], 200);
    }

    #[test]
    fn test_apply_filter_none_does_nothing() {
        let mut data = vec![100, 150, 200, 255];
        DocumentStore::apply_filter(&mut data, RenderFilter::None);
        assert_eq!(data[0], 100);
        assert_eq!(data[1], 150);
        assert_eq!(data[2], 200);
    }

    #[test]
    fn test_apply_filter_large_buffer() {
        let mut data = vec![0u8; 10000];
        for i in 0..2500 {
            data[i * 4] = 100;
            data[i * 4 + 1] = 150;
            data[i * 4 + 2] = 200;
            data[i * 4 + 3] = 255;
        }
        DocumentStore::apply_filter(&mut data, RenderFilter::Inverted);
        for i in 0..2500 {
            assert_eq!(data[i * 4], 155);
            assert_eq!(data[i * 4 + 1], 105);
            assert_eq!(data[i * 4 + 2], 55);
        }
    }

    #[test]
    fn test_detect_content_bbox_parallel_empty() {
        let data = vec![255u8; 40];
        let result = DocumentStore::detect_content_bbox_parallel(&data, 10, 10);
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_content_bbox_parallel_full() {
        let mut data = vec![255u8; 40];
        data[0] = 100;
        data[4] = 100;
        let result = DocumentStore::detect_content_bbox_parallel(&data, 10, 10);
        assert!(result.is_some());
        let (min_x, min_y, max_x, max_y) = result.unwrap();
        assert!(min_x <= max_x);
        assert!(min_y <= max_y);
    }

    #[test]
    fn test_detect_content_bbox_parallel_with_margin() {
        let mut data = vec![255u8; 40];
        data[0] = 100;
        let result = DocumentStore::detect_content_bbox_parallel(&data, 10, 10);
        assert!(result.is_some());
        let (min_x, min_y, max_x, max_y) = result.unwrap();
        assert!(min_x <= 10);
        assert!(min_y <= 10);
        assert!(max_x >= 0);
        assert!(max_y >= 0);
    }

    #[test]
    fn test_create_render_cache_defaults() {
        let cache = create_render_cache(10, 0);
        assert!(
            cache
                .get(&RenderKey {
                    doc_id: DocumentId(1),
                    page_num: 0,
                    scale: 100,
                    filter: RenderFilter::None,
                    auto_crop: false,
                    quality: RenderQuality::Medium,
                })
                .is_none()
        );
    }

    #[test]
    fn test_render_options_default() {
        let options = RenderOptions {
            scale: 1.0,
            rotation: 0,
            filter: RenderFilter::None,
            auto_crop: false,
            quality: RenderQuality::Medium,
        };
        assert_eq!(options.scale, 1.0);
        assert_eq!(options.rotation, 0);
    }

    #[test]
    fn test_render_quality_serialization() {
        let json_low = serde_json::to_string(&RenderQuality::Low).unwrap();
        let json_medium = serde_json::to_string(&RenderQuality::Medium).unwrap();
        let json_high = serde_json::to_string(&RenderQuality::High).unwrap();
        assert_eq!(json_low, "\"Low\"");
        assert_eq!(json_medium, "\"Medium\"");
        assert_eq!(json_high, "\"High\"");
    }

    #[test]
    fn test_render_filter_serialization() {
        let json_none = serde_json::to_string(&RenderFilter::None).unwrap();
        let json_grayscale = serde_json::to_string(&RenderFilter::Grayscale).unwrap();
        let json_inverted = serde_json::to_string(&RenderFilter::Inverted).unwrap();
        assert_eq!(json_none, "\"None\"");
        assert_eq!(json_grayscale, "\"Grayscale\"");
        assert_eq!(json_inverted, "\"Inverted\"");
    }

    #[test]
    fn test_render_quality_deserialization() {
        let low: RenderQuality = serde_json::from_str("\"Low\"").unwrap();
        let medium: RenderQuality = serde_json::from_str("\"Medium\"").unwrap();
        let high: RenderQuality = serde_json::from_str("\"High\"").unwrap();
        assert_eq!(low, RenderQuality::Low);
        assert_eq!(medium, RenderQuality::Medium);
        assert_eq!(high, RenderQuality::High);
    }

    #[test]
    fn test_render_filter_deserialization() {
        let none: RenderFilter = serde_json::from_str("\"None\"").unwrap();
        let grayscale: RenderFilter = serde_json::from_str("\"Grayscale\"").unwrap();
        let inverted: RenderFilter = serde_json::from_str("\"Inverted\"").unwrap();
        assert_eq!(none, RenderFilter::None);
        assert_eq!(grayscale, RenderFilter::Grayscale);
        assert_eq!(inverted, RenderFilter::Inverted);
    }
}
