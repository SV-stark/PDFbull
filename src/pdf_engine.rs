use crate::models::{
    Annotation, AnnotationStyle, DocumentId, EngineErrorKind, FormField, FormFieldVariant,
    Hyperlink, PdfError, PdfResult, SearchResultItem,
};
use lopdf::{Document, Object, ObjectId};
use quick_cache::{Weighter, sync::Cache};
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use zpdf::{
    ContentInterpreter, FieldKind, FieldValue, ImageCache, PdfDocument, RenderBackend,
    gpu::WgpuRenderer, spans_to_text,
};
use zune_image::codecs::ImageFormat;
use zune_image::image::Image;

use crate::ui::theme::hex_to_rgb;

// PDF field-flags bit for "radio button" (ISO 32000-1 Table 221).
const FF_RADIO: i64 = 1 << 15;
const WHITE_THRESHOLD: u8 = 245;
const BBOX_MARGIN: u32 = 10;
const NO_SHADOW_THRESHOLD: u8 = 230;

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct RenderKey {
    pub doc_id: DocumentId,
    pub page_num: usize,
    pub scale: u32,
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
    Sepia,
}

#[derive(Debug, Clone)]
pub struct RenderOptions {
    pub scale: f32,
    pub rotation: i32,
    pub filter: RenderFilter,
    pub auto_crop: bool,
    pub quality: RenderQuality,
}

pub struct DocumentStore {
    documents: HashMap<DocumentId, PdfDocument>,
    paths: HashMap<DocumentId, String>,
    render_cache: SharedRenderCache,
}

// DocumentState wrapper removed as it was a single-field struct.

impl DocumentStore {
    pub fn new(cache: SharedRenderCache) -> Self {
        Self {
            documents: HashMap::new(),
            paths: HashMap::new(),
            render_cache: cache,
        }
    }

    pub fn has_document(&self, doc_id: DocumentId) -> bool {
        self.documents.contains_key(&doc_id)
    }

    pub fn open_document(
        &mut self,
        path: &str,
        doc_id: DocumentId,
    ) -> PdfResult<crate::models::OpenResult> {
        let data = std::fs::read(path).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        let doc = PdfDocument::open(data).map_err(|e| PdfError::OpenFailed(e.to_string()))?;

        let page_count = doc.page_count();
        let mut heights = Vec::with_capacity(page_count);
        let mut max_width = 0.0;

        for i in 0..page_count {
            if let Ok(page) = doc.page(i) {
                let rect = page.effective_box();
                let w = rect.width() as f32;
                let h = rect.height() as f32;
                heights.push(h);
                if w > max_width {
                    max_width = w;
                }
            } else {
                heights.push(0.0);
            }
        }

        self.documents.insert(doc_id, doc);
        self.paths.insert(doc_id, path.to_string());

        Ok(crate::models::OpenResult {
            id: doc_id,
            page_count,
            page_heights: heights,
            max_width,
            outline: Vec::new(),
            links: Vec::new(),
            metadata: crate::models::DocumentMetadata::default(),
        })
    }

    fn extract_links_internal(&self, doc: &PdfDocument) -> Vec<Hyperlink> {
        let mut all_links = Vec::new();
        let page_count = doc.page_count();

        for i in 0..page_count {
            if let Ok(page) = doc.page(i) {
                let annots = doc.page_annotations(&page);
                for annot in annots {
                    if annot.subtype == "Link" {
                        let rect = annot.rect;
                        let url = annot.uri.clone();
                        let dest = annot.dest.as_ref().and_then(|d| d.page);
                        if url.is_some() || dest.is_some() {
                            all_links.push(Hyperlink {
                                page: i,
                                bounds: (
                                    rect.x0 as f32,
                                    rect.y0 as f32,
                                    rect.width() as f32,
                                    rect.height() as f32,
                                ),
                                url,
                                destination_page: dest,
                            });
                        }
                    }
                }
            }
        }
        all_links
    }

    pub fn load_document_meta(&self, doc_id: DocumentId) -> PdfResult<crate::models::DocumentMeta> {
        let doc = self
            .documents
            .get(&doc_id)
            .ok_or(PdfError::EngineError(EngineErrorKind::DocumentNotFound))?;

        let outline = self.get_outline_internal(doc);
        let links = self.extract_links_internal(doc);

        let metadata = if let Some(info) = doc.info() {
            crate::models::DocumentMetadata {
                title: info.title.clone(),
                author: info.author.clone(),
                subject: info.subject.clone(),
                keywords: info.keywords.clone(),
                creator: info.creator.clone(),
                producer: info.producer.clone(),
                creation_date: info.creation_date.clone(),
                modification_date: info.mod_date.clone(),
            }
        } else {
            crate::models::DocumentMetadata::default()
        };

        Ok(crate::models::DocumentMeta {
            outline,
            links,
            metadata,
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

        if let Some(base) = self.render_cache.get(&cache_key) {
            if options.filter == RenderFilter::None {
                return Ok(base);
            }
            let mut filtered = base.data.to_vec();
            Self::apply_filter(&mut filtered, options.filter);
            return Ok(crate::models::RenderResult {
                width: base.width,
                height: base.height,
                data: filtered.into(),
            });
        }

        let doc = self
            .documents
            .get(&doc_id)
            .ok_or(PdfError::EngineError(EngineErrorKind::DocumentNotFound))?;
        let page = doc
            .page(page_num)
            .map_err(|_| PdfError::PageNotFound(page_num))?;

        let mut fonts = doc.load_page_fonts(&page);
        let mut images = ImageCache::new();
        let content = doc
            .page_content_bytes(&page)
            .map_err(|e| PdfError::RenderFailed(e.to_string()))?;

        // Incorporate custom option rotation into the display list rotation
        let display_list = ContentInterpreter::new(page.effective_box())
            .with_page_rotation(page.rotate + options.rotation)
            .with_fonts(&mut fonts)
            .with_document(doc.file(), &page.resources)
            .with_images(&mut images)
            .interpret(&content);

        let mut renderer = WgpuRenderer::new().with_fonts(&fonts).with_images(&images);
        let page_img = renderer
            .render_display_list(&display_list, options.scale)
            .map_err(|e| PdfError::RenderFailed(e.to_string()))?;
        let w = page_img.width;
        let h = page_img.height;

        let (final_w, final_h, final_data) = if !is_thumbnail && options.auto_crop {
            let result_data = page_img.data;

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
            (w, h, page_img.data)
        };

        let base = crate::models::RenderResult {
            width: final_w,
            height: final_h,
            data: final_data.into(),
        };

        self.render_cache.put(cache_key, base.clone());

        if options.filter == RenderFilter::None {
            Ok(base)
        } else {
            let mut filtered = base.data.to_vec();
            Self::apply_filter(&mut filtered, options.filter);
            Ok(crate::models::RenderResult {
                width: base.width,
                height: base.height,
                data: filtered.into(),
            })
        }
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
            .page(page_num as usize)
            .map_err(|_| PdfError::PageNotFound(page_num as usize))?;
        let mut fonts = doc.load_page_fonts(&page);
        let mut images = ImageCache::new();
        let content = doc
            .page_content_bytes(&page)
            .map_err(|e| PdfError::SearchError(e.to_string()))?;

        let mut spans = Vec::new();
        {
            let interp = ContentInterpreter::new(page.effective_box())
                .with_fonts(&mut fonts)
                .with_document(doc.file(), &page.resources)
                .with_images(&mut images)
                .with_text_sink(&mut spans);
            let _ = interp.interpret(&content);
        }
        let text = spans_to_text(spans, 2.0);
        Ok(text)
    }

    pub fn extract_text_items(
        &self,
        doc_id: DocumentId,
        page_num: usize,
    ) -> PdfResult<Vec<crate::models::TextItem>> {
        let doc = self
            .documents
            .get(&doc_id)
            .ok_or(PdfError::EngineError(EngineErrorKind::DocumentNotFound))?;
        let page = doc
            .page(page_num)
            .map_err(|_| PdfError::PageNotFound(page_num))?;
        let mut fonts = doc.load_page_fonts(&page);
        let mut images = ImageCache::new();
        let content = doc
            .page_content_bytes(&page)
            .map_err(|e| PdfError::SearchError(e.to_string()))?;

        let mut spans = Vec::new();
        {
            let interp = ContentInterpreter::new(page.effective_box())
                .with_fonts(&mut fonts)
                .with_document(doc.file(), &page.resources)
                .with_images(&mut images)
                .with_text_sink(&mut spans);
            let _ = interp.interpret(&content);
        }

        let page_height = page.effective_box().height() as f32;
        let mut text_items = Vec::new();
        for span in spans {
            if span.text.trim().is_empty() {
                continue;
            }
            text_items.push(crate::models::TextItem {
                text: span.text,
                x: span.x as f32,
                y: page_height - span.y as f32,
                width: span.advance.abs() as f32,
                height: span.size,
            });
        }
        Ok(text_items)
    }

    #[allow(clippy::suboptimal_flops)]
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

        let mut doc = Document::load(pdf_path).map_err(|e| PdfError::OpenFailed(e.to_string()))?;

        // Group annotations by page
        let mut page_annots: HashMap<usize, Vec<&Annotation>> = HashMap::new();
        for ann in annotations {
            page_annots.entry(ann.page).or_default().push(ann);
        }

        // Get the page object IDs in the document
        let pages = doc.get_pages();

        for (page_idx, annotations) in page_annots {
            let page_key = (page_idx + 1) as u32;
            let Some(&page_id) = pages.get(&page_key) else {
                continue;
            };

            let page_height = {
                if let Some(doc_ref) = self.documents.get(&doc_id) {
                    if let Ok(p) = doc_ref.page(page_idx) {
                        p.effective_box().height() as f32
                    } else {
                        792.0_f32
                    }
                } else {
                    792.0_f32
                }
            };

            let mut annot_refs = Vec::new();

            for ann in annotations {
                let pdf_x = ann.x as f32;
                let pdf_w = ann.width as f32;
                let pdf_h = ann.height as f32;
                let pdf_y = page_height - (ann.y as f32 + pdf_h);

                let mut annot_dict = lopdf::Dictionary::new();
                annot_dict.set("Type", Object::Name(b"Annot".to_vec()));

                match &ann.style {
                    AnnotationStyle::Highlight { color } => {
                        let (r, g, b) = hex_to_rgb(color);
                        annot_dict.set("Subtype", Object::Name(b"Highlight".to_vec()));
                        annot_dict.set(
                            "Rect",
                            Object::Array(vec![
                                Object::Real(pdf_x),
                                Object::Real(pdf_y),
                                Object::Real(pdf_x + pdf_w),
                                Object::Real(pdf_y + pdf_h),
                            ]),
                        );
                        annot_dict.set(
                            "C",
                            Object::Array(vec![
                                Object::Real(r as f32),
                                Object::Real(g as f32),
                                Object::Real(b as f32),
                            ]),
                        );
                        annot_dict.set(
                            "QuadPoints",
                            Object::Array(vec![
                                Object::Real(pdf_x),
                                Object::Real(pdf_y + pdf_h),
                                Object::Real(pdf_x + pdf_w),
                                Object::Real(pdf_y + pdf_h),
                                Object::Real(pdf_x),
                                Object::Real(pdf_y),
                                Object::Real(pdf_x + pdf_w),
                                Object::Real(pdf_y),
                            ]),
                        );
                    }
                    AnnotationStyle::Rectangle { color, fill, .. } => {
                        let (r, g, b) = hex_to_rgb(color);
                        annot_dict.set("Subtype", Object::Name(b"Square".to_vec()));
                        annot_dict.set(
                            "Rect",
                            Object::Array(vec![
                                Object::Real(pdf_x),
                                Object::Real(pdf_y),
                                Object::Real(pdf_x + pdf_w),
                                Object::Real(pdf_y + pdf_h),
                            ]),
                        );
                        annot_dict.set(
                            "C",
                            Object::Array(vec![
                                Object::Real(r as f32),
                                Object::Real(g as f32),
                                Object::Real(b as f32),
                            ]),
                        );
                        if *fill {
                            annot_dict.set(
                                "IC",
                                Object::Array(vec![
                                    Object::Real(r as f32),
                                    Object::Real(g as f32),
                                    Object::Real(b as f32),
                                ]),
                            );
                        }
                    }
                    AnnotationStyle::Circle { color, fill, .. } => {
                        let (r, g, b) = hex_to_rgb(color);
                        annot_dict.set("Subtype", Object::Name(b"Circle".to_vec()));
                        annot_dict.set(
                            "Rect",
                            Object::Array(vec![
                                Object::Real(pdf_x),
                                Object::Real(pdf_y),
                                Object::Real(pdf_x + pdf_w),
                                Object::Real(pdf_y + pdf_h),
                            ]),
                        );
                        annot_dict.set(
                            "C",
                            Object::Array(vec![
                                Object::Real(r as f32),
                                Object::Real(g as f32),
                                Object::Real(b as f32),
                            ]),
                        );
                        if *fill {
                            annot_dict.set(
                                "IC",
                                Object::Array(vec![
                                    Object::Real(r as f32),
                                    Object::Real(g as f32),
                                    Object::Real(b as f32),
                                ]),
                            );
                        }
                    }
                    AnnotationStyle::Text { text, color, .. } => {
                        let (r, g, b) = hex_to_rgb(color);
                        annot_dict.set("Subtype", Object::Name(b"FreeText".to_vec()));
                        annot_dict.set(
                            "Rect",
                            Object::Array(vec![
                                Object::Real(pdf_x),
                                Object::Real(pdf_y),
                                Object::Real(pdf_x + pdf_w),
                                Object::Real(pdf_y + pdf_h),
                            ]),
                        );
                        annot_dict.set("Contents", Object::string_literal(text.clone()));
                        annot_dict.set(
                            "C",
                            Object::Array(vec![
                                Object::Real(r as f32),
                                Object::Real(g as f32),
                                Object::Real(b as f32),
                            ]),
                        );
                    }
                    AnnotationStyle::StickyNote { comment, color } => {
                        let (r, g, b) = hex_to_rgb(color);
                        annot_dict.set("Subtype", Object::Name(b"Text".to_vec()));
                        annot_dict.set(
                            "Rect",
                            Object::Array(vec![
                                Object::Real(pdf_x),
                                Object::Real(pdf_y),
                                Object::Real(pdf_x + 30.0),
                                Object::Real(pdf_y + 30.0),
                            ]),
                        );
                        annot_dict.set("Contents", Object::string_literal(comment.clone()));
                        annot_dict.set(
                            "C",
                            Object::Array(vec![
                                Object::Real(r as f32),
                                Object::Real(g as f32),
                                Object::Real(b as f32),
                            ]),
                        );
                    }
                    AnnotationStyle::Redact { color } => {
                        let (r, g, b) = hex_to_rgb(color);
                        annot_dict.set("Subtype", Object::Name(b"Square".to_vec()));
                        annot_dict.set(
                            "Rect",
                            Object::Array(vec![
                                Object::Real(pdf_x),
                                Object::Real(pdf_y),
                                Object::Real(pdf_x + pdf_w),
                                Object::Real(pdf_y + pdf_h),
                            ]),
                        );
                        annot_dict.set(
                            "C",
                            Object::Array(vec![
                                Object::Real(r as f32),
                                Object::Real(g as f32),
                                Object::Real(b as f32),
                            ]),
                        );
                        annot_dict.set(
                            "IC",
                            Object::Array(vec![
                                Object::Real(r as f32),
                                Object::Real(g as f32),
                                Object::Real(b as f32),
                            ]),
                        );
                    }
                    AnnotationStyle::Line { color, thickness } => {
                        let (r, g, b) = hex_to_rgb(color);
                        annot_dict.set("Subtype", Object::Name(b"Line".to_vec()));
                        annot_dict.set(
                            "Rect",
                            Object::Array(vec![
                                Object::Real(pdf_x.min(pdf_x + pdf_w)),
                                Object::Real(pdf_y.min(pdf_y + pdf_h)),
                                Object::Real(pdf_x.max(pdf_x + pdf_w)),
                                Object::Real(pdf_y.max(pdf_y + pdf_h)),
                            ]),
                        );
                        let x1 = ann.x as f32;
                        let y1 = page_height - ann.y as f32;
                        let x2 = (ann.x + ann.width) as f32;
                        let y2 = page_height - (ann.y + ann.height) as f32;
                        annot_dict.set(
                            "L",
                            Object::Array(vec![
                                Object::Real(x1),
                                Object::Real(y1),
                                Object::Real(x2),
                                Object::Real(y2),
                            ]),
                        );
                        annot_dict.set(
                            "C",
                            Object::Array(vec![
                                Object::Real(r as f32),
                                Object::Real(g as f32),
                                Object::Real(b as f32),
                            ]),
                        );
                        let border = lopdf::Dictionary::from_iter(vec![(
                            "W",
                            Object::Real(*thickness as f32),
                        )]);
                        annot_dict.set("BS", Object::Dictionary(border));
                    }
                    AnnotationStyle::Arrow { color, thickness } => {
                        let (r, g, b) = hex_to_rgb(color);
                        annot_dict.set("Subtype", Object::Name(b"Line".to_vec()));
                        annot_dict.set(
                            "Rect",
                            Object::Array(vec![
                                Object::Real(pdf_x.min(pdf_x + pdf_w)),
                                Object::Real(pdf_y.min(pdf_y + pdf_h)),
                                Object::Real(pdf_x.max(pdf_x + pdf_w)),
                                Object::Real(pdf_y.max(pdf_y + pdf_h)),
                            ]),
                        );
                        let x1 = ann.x as f32;
                        let y1 = page_height - ann.y as f32;
                        let x2 = (ann.x + ann.width) as f32;
                        let y2 = page_height - (ann.y + ann.height) as f32;
                        annot_dict.set(
                            "L",
                            Object::Array(vec![
                                Object::Real(x1),
                                Object::Real(y1),
                                Object::Real(x2),
                                Object::Real(y2),
                            ]),
                        );
                        annot_dict.set(
                            "C",
                            Object::Array(vec![
                                Object::Real(r as f32),
                                Object::Real(g as f32),
                                Object::Real(b as f32),
                            ]),
                        );
                        annot_dict.set(
                            "LE",
                            Object::Array(vec![
                                Object::Name(b"None".to_vec()),
                                Object::Name(b"ClosedArrow".to_vec()),
                            ]),
                        );
                        let border = lopdf::Dictionary::from_iter(vec![(
                            "W",
                            Object::Real(*thickness as f32),
                        )]);
                        annot_dict.set("BS", Object::Dictionary(border));
                    }
                }

                let annot_id = doc.add_object(Object::Dictionary(annot_dict));
                annot_refs.push(Object::Reference(annot_id));
            }

            let existing_annots = doc
                .objects
                .get(&page_id)
                .and_then(|o| o.as_dict().ok())
                .and_then(|d| d.get(b"Annots").ok())
                .and_then(|a| match a {
                    Object::Reference(r) => doc.objects.get(r).and_then(|o| o.as_array().ok()),
                    Object::Array(arr) => Some(arr),
                    _ => None,
                })
                .cloned();

            if let Some(Object::Dictionary(page_dict)) = doc.objects.get_mut(&page_id) {
                let mut annots_resolved = existing_annots.unwrap_or_default();
                annots_resolved.extend(annot_refs);
                page_dict.set("Annots", Object::Array(annots_resolved));
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

        doc.save(&final_path)
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
            .page(page_num as usize)
            .map_err(|_| PdfError::PageNotFound(page_num as usize))?;

        let mut fonts = doc.load_page_fonts(&page);
        let mut images = ImageCache::new();
        let content = doc
            .page_content_bytes(&page)
            .map_err(|e| PdfError::RenderFailed(e.to_string()))?;

        let display_list = ContentInterpreter::new(page.effective_box())
            .with_page_rotation(page.rotate)
            .with_fonts(&mut fonts)
            .with_document(doc.file(), &page.resources)
            .with_images(&mut images)
            .interpret(&content);

        let mut renderer = WgpuRenderer::new().with_fonts(&fonts).with_images(&images);
        let page_img = renderer
            .render_display_list(&display_list, scale)
            .map_err(|e| PdfError::RenderFailed(e.to_string()))?;
        let width = page_img.width as usize;
        let height = page_img.height as usize;

        let image = Image::from_u8(
            &page_img.data,
            width,
            height,
            zune_core::colorspace::ColorSpace::RGBA,
        );
        let out_buf = image
            .write_to_vec(ImageFormat::PNG)
            .map_err(|e| PdfError::RenderFailed(format!("{e:?}")))?;

        Ok(out_buf)
    }

    fn flatten_outline(items: &[zpdf::OutlineItem], out: &mut Vec<Bookmark>) {
        for item in items {
            let page_idx = item.dest.as_ref().and_then(|d| d.page).unwrap_or(0);
            out.push(Bookmark {
                title: item.title.clone(),
                page_index: page_idx as u16,
            });
            Self::flatten_outline(&item.children, out);
        }
    }

    pub fn get_outline_internal(&self, doc: &PdfDocument) -> Vec<Bookmark> {
        let mut bookmarks = Vec::new();
        Self::flatten_outline(&doc.outline(), &mut bookmarks);
        bookmarks
    }

    pub fn search(&self, doc_id: DocumentId, query: &str) -> PdfResult<Vec<SearchResultItem>> {
        let path = self
            .paths
            .get(&doc_id)
            .ok_or(PdfError::EngineError(EngineErrorKind::DocumentPathNotFound))?;
        let file_data = std::fs::read(path).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        let shared_data: std::sync::Arc<[u8]> = std::sync::Arc::from(file_data.into_boxed_slice());

        let doc = self
            .documents
            .get(&doc_id)
            .ok_or(PdfError::EngineError(EngineErrorKind::DocumentNotFound))?;
        let total_pages = doc.page_count();
        let query_lower = query.to_lowercase();

        let results: Vec<SearchResultItem> = (0..total_pages)
            .into_par_iter()
            .flat_map(|page_idx| {
                let local_bytes = shared_data.to_vec();
                let Ok(local_doc) = PdfDocument::open(local_bytes) else {
                    return Vec::new();
                };
                let Ok(page) = local_doc.page(page_idx) else {
                    return Vec::new();
                };
                let mut fonts = local_doc.load_page_fonts(&page);
                let mut images = ImageCache::new();
                let Ok(content) = local_doc.page_content_bytes(&page) else {
                    return Vec::new();
                };

                let mut spans: Vec<zpdf::TextSpan> = Vec::new();
                {
                    let interp = ContentInterpreter::new(page.effective_box())
                        .with_fonts(&mut fonts)
                        .with_document(local_doc.file(), &page.resources)
                        .with_images(&mut images)
                        .with_text_sink(&mut spans);
                    let _ = interp.interpret(&content);
                }

                let page_height = page.effective_box().height() as f32;
                let mut full_text = String::new();
                let mut span_offsets = Vec::new();

                for (idx, span) in spans.iter().enumerate() {
                    let start = full_text.len();
                    full_text.push_str(&span.text);
                    let end = full_text.len();
                    span_offsets.push((start, end, idx));
                }

                let mut page_results = Vec::new();
                let mut search_idx = 0;
                while let Some(pos) = full_text.to_lowercase()[search_idx..].find(&query_lower) {
                    let match_start = search_idx + pos;
                    let match_end = match_start + query_lower.len();

                    if let Some(&(_, _, span_idx)) = span_offsets
                        .iter()
                        .find(|(s, e, _)| match_start >= *s && match_start < *e)
                    {
                        let first_span = &spans[span_idx];
                        let y_top_down = page_height - first_span.y as f32 - first_span.size;
                        page_results.push(SearchResultItem {
                            page_index: page_idx,
                            text: full_text[match_start..match_end].to_string(),
                            y: y_top_down,
                            x: first_span.x as f32,
                            width: first_span.advance.abs() as f32,
                            height: first_span.size,
                        });
                    }

                    search_idx = match_start + 1;
                }
                page_results
            })
            .collect();

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

    #[allow(clippy::suboptimal_flops)]
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
            RenderFilter::NoShadow => {
                if pixel[0] > NO_SHADOW_THRESHOLD
                    && pixel[1] > NO_SHADOW_THRESHOLD
                    && pixel[2] > NO_SHADOW_THRESHOLD
                {
                    pixel[0] = 255;
                    pixel[1] = 255;
                    pixel[2] = 255;
                }
            }
            RenderFilter::Sepia => {
                let r = pixel[0] as f32;
                let g = pixel[1] as f32;
                let b = pixel[2] as f32;
                pixel[0] = (r * 0.393 + g * 0.769 + b * 0.189).min(255.0) as u8;
                pixel[1] = (r * 0.349 + g * 0.686 + b * 0.168).min(255.0) as u8;
                pixel[2] = (r * 0.272 + g * 0.534 + b * 0.131).min(255.0) as u8;
            }
            RenderFilter::Grayscale => {
                let luma =
                    (pixel[0] as u32 * 299 + pixel[1] as u32 * 587 + pixel[2] as u32 * 114) / 1000;
                pixel[0] = luma as u8;
                pixel[1] = luma as u8;
                pixel[2] = luma as u8;
            }
            RenderFilter::None => {}
        });
    }

    // apply_filter_parallel removed as it was just a misleading wrapper.

    pub fn optimize_pdf(&self, input_path: &str, output_path: &str) -> PdfResult<String> {
        let mut doc =
            Document::load(input_path).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        doc.compress();
        doc.prune_objects();
        let _ = doc.trailer.remove(b"Info");
        doc.save(output_path)
            .map_err(|e| PdfError::IoError(e.to_string()))?;
        Ok(output_path.to_string())
    }

    pub fn merge_documents(&self, paths: Vec<String>, output_path: String) -> PdfResult<String> {
        let mut max_id = 1;
        let mut documents = Vec::new();

        for path in paths {
            let mut doc = Document::load(&path).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
            doc.renumber_objects_with(max_id);
            max_id = doc.max_id + 1;
            documents.push(doc);
        }

        if documents.is_empty() {
            return Err(PdfError::IoError("No documents to merge".into()));
        }

        let mut merged_doc = Document::with_version("1.5");
        let mut merged_kids = Vec::new();
        let mut merged_objects = std::collections::BTreeMap::new();

        for doc in &documents {
            merged_objects.extend(doc.objects.clone());
            let pages = doc.get_pages();
            for (_, page_id) in pages {
                merged_kids.push(Object::Reference(page_id));
            }
        }

        let pages_id = max_id;
        max_id += 1;

        let count = merged_kids.len() as i32;
        let pages_dict = lopdf::Dictionary::from_iter(vec![
            ("Type", Object::Name(b"Pages".to_vec())),
            ("Count", Object::Integer(count as i64)),
            ("Kids", Object::Array(merged_kids)),
        ]);
        merged_objects.insert((pages_id, 0), Object::Dictionary(pages_dict));

        let catalog_id = max_id;
        max_id += 1;
        let catalog_dict = lopdf::Dictionary::from_iter(vec![
            ("Type", Object::Name(b"Catalog".to_vec())),
            ("Pages", Object::Reference((pages_id, 0))),
        ]);
        merged_objects.insert((catalog_id, 0), Object::Dictionary(catalog_dict));

        for doc in &documents {
            let pages = doc.get_pages();
            for (_, page_id) in pages {
                if let Some(Object::Dictionary(dict)) = merged_objects.get_mut(&page_id) {
                    dict.set("Parent", Object::Reference((pages_id, 0)));
                }
            }
        }

        merged_doc.objects = merged_objects;
        merged_doc
            .trailer
            .set("Root", Object::Reference((catalog_id, 0)));
        merged_doc.trailer.set("Size", max_id as i64);
        merged_doc.max_id = max_id - 1;

        merged_doc
            .save(&output_path)
            .map_err(|e| PdfError::IoError(e.to_string()))?;

        Ok(output_path)
    }

    pub fn reorder_pages(
        &self,
        input_path: &str,
        page_order: &[usize],
        output_path: &str,
    ) -> PdfResult<String> {
        let mut doc = lopdf::Document::load(input_path)
            .map_err(|e| PdfError::from(format!("Failed to load PDF for reorder: {e}")))?;

        let pages: Vec<lopdf::ObjectId> = doc.page_iter().collect();
        let total = pages.len();

        // Build the new page list in the requested order (filter out-of-range indices)
        let reordered: Vec<lopdf::ObjectId> = page_order
            .iter()
            .filter(|&&i| i < total)
            .map(|&i| pages[i])
            .collect();

        if reordered.is_empty() {
            return Err(PdfError::from("No valid pages in reorder mapping"));
        }

        // Get or build the Pages root
        let pages_root_id = doc.get_pages().values().next().and_then(|&oid| {
            doc.get_object(oid)
                .ok()
                .and_then(|o| o.as_dict().ok())
                .and_then(|d| d.get(b"Parent").ok())
                .and_then(|p| p.as_reference().ok())
        });

        if let Some(root_id) = pages_root_id {
            // Update Kids array on the Pages root
            if let Ok(lopdf::Object::Dictionary(dict)) = doc.get_object_mut(root_id) {
                let kids: Vec<lopdf::Object> = reordered
                    .iter()
                    .map(|&id| lopdf::Object::Reference(id))
                    .collect();
                dict.set(b"Kids", lopdf::Object::Array(kids));
                dict.set(b"Count", lopdf::Object::Integer(reordered.len() as i64));
            }
        }

        doc.save(output_path)
            .map_err(|e| PdfError::from(format!("Failed to save reordered PDF: {e}")))?;

        Ok(output_path.to_string())
    }

    pub fn split_pdf(
        &self,
        path: &str,
        page_indices: Vec<usize>,
        output_dir: String,
    ) -> PdfResult<Vec<String>> {
        let mut created_paths = Vec::new();

        for &page_idx in &page_indices {
            let mut doc = Document::load(path).map_err(|e| PdfError::OpenFailed(e.to_string()))?;

            let page_count = doc.get_pages().len();
            let keep_page_1_based = (page_idx + 1) as u32;

            let mut to_delete = Vec::new();
            for p in 1..=(page_count as u32) {
                if p != keep_page_1_based {
                    to_delete.push(p);
                }
            }

            doc.delete_pages(&to_delete);

            let filename = std::path::Path::new(path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("document");

            let out_path = format!("{}/{}_page_{}.pdf", output_dir, filename, page_idx + 1);
            doc.save(&out_path)
                .map_err(|e| PdfError::IoError(e.to_string()))?;
            created_paths.push(out_path);
        }

        Ok(created_paths)
    }

    pub fn get_form_fields(&mut self, path: &str) -> PdfResult<Vec<FormField>> {
        let data = std::fs::read(path).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        let doc = PdfDocument::open(data).map_err(|e| PdfError::OpenFailed(e.to_string()))?;

        let mut fields = Vec::new();
        if let Some(acro) = doc.acro_form() {
            for f in &acro.fields {
                let name = f.name.clone();
                let variant = match f.kind {
                    FieldKind::Text => {
                        let val = match &f.value {
                            Some(FieldValue::Text(s)) => s.clone(),
                            _ => String::new(),
                        };
                        FormFieldVariant::Text { value: val }
                    }
                    FieldKind::Button => {
                        let is_checked = match &f.value {
                            Some(FieldValue::Name(n)) => n != "Off",
                            _ => false,
                        };
                        if f.flags & FF_RADIO != 0 {
                            FormFieldVariant::RadioButton {
                                is_selected: is_checked,
                                group_name: Some(name.clone()),
                            }
                        } else {
                            FormFieldVariant::Checkbox { is_checked }
                        }
                    }
                    FieldKind::Choice => {
                        let opts: Vec<String> =
                            f.options.iter().map(|(_, label)| label.clone()).collect();
                        let selected_val = match &f.value {
                            Some(FieldValue::Text(s)) => Some(s.clone()),
                            _ => None,
                        };
                        let selected_index = selected_val.and_then(|val| {
                            f.options.iter().position(|(export, _)| *export == val)
                        });
                        FormFieldVariant::ComboBox {
                            options: opts,
                            selected_index,
                        }
                    }
                    _ => FormFieldVariant::Text {
                        value: String::new(),
                    },
                };

                let mut page_idx = 0;
                if let Some(&widget_id) = f.widgets.first() {
                    for i in 0..doc.page_count() {
                        if let Ok(page) = doc.page(i) {
                            if page.annots.contains(&widget_id) {
                                page_idx = i;
                                break;
                            }
                        }
                    }
                }

                fields.push(FormField {
                    name,
                    variant,
                    page: page_idx,
                });
            }
        }
        Ok(fields)
    }

    fn walk_lopdf_fields(
        doc: &mut Document,
        field_ref: ObjectId,
        parent_name: &str,
        updates: &[FormField],
    ) {
        let mut name = parent_name.to_string();

        let field_dict = match doc.get_object(field_ref) {
            Ok(Object::Dictionary(dict)) => dict.clone(),
            _ => return,
        };

        if let Ok(partial_name_obj) = field_dict.get(b"T") {
            if let Ok(partial_name) = partial_name_obj.as_str() {
                let partial_str = String::from_utf8_lossy(partial_name).into_owned();
                if name.is_empty() {
                    name = partial_str;
                } else {
                    name = format!("{name}.{partial_str}");
                }
            }
        }

        if let Ok(kids_obj) = field_dict.get(b"Kids") {
            if let Ok(kids_arr) = kids_obj.as_array() {
                for kid in kids_arr {
                    if let Ok(kid_ref) = kid.as_reference() {
                        Self::walk_lopdf_fields(doc, kid_ref, &name, updates);
                    }
                }
                return;
            }
        }

        if let Some(update) = updates.iter().find(|u| u.name == name) {
            if let Some(Object::Dictionary(dict)) = doc.objects.get_mut(&field_ref) {
                match &update.variant {
                    FormFieldVariant::Text { value } => {
                        dict.set("V", Object::string_literal(value.clone()));
                    }
                    FormFieldVariant::Checkbox { is_checked } => {
                        let name_val = if *is_checked { "Yes" } else { "Off" };
                        dict.set("V", Object::Name(name_val.as_bytes().to_vec()));
                        dict.set("AS", Object::Name(name_val.as_bytes().to_vec()));
                    }
                    FormFieldVariant::RadioButton { is_selected, .. } => {
                        if *is_selected {
                            dict.set("V", Object::Name(b"Yes".to_vec()));
                            dict.set("AS", Object::Name(b"Yes".to_vec()));
                        } else {
                            dict.set("V", Object::Name(b"Off".to_vec()));
                            dict.set("AS", Object::Name(b"Off".to_vec()));
                        }
                    }
                    FormFieldVariant::ComboBox {
                        selected_index,
                        options,
                    } => {
                        if let Some(idx) = selected_index {
                            if let Some(opt_text) = options.get(*idx) {
                                dict.set("V", Object::string_literal(opt_text.clone()));
                            }
                        }
                    }
                }
                if let Ok(catalog) = doc.catalog_mut() {
                    let acro_ref = catalog
                        .get(b"AcroForm")
                        .ok()
                        .and_then(|o| o.as_reference().ok());
                    let _ = catalog;
                    if let Some(acro_ref) = acro_ref {
                        if let Some(Object::Dictionary(acro)) = doc.objects.get_mut(&acro_ref) {
                            acro.set("NeedAppearances", Object::Boolean(true));
                        }
                    }
                }
            }
        }
    }

    pub fn fill_form(
        &mut self,
        path: &str,
        updates: Vec<FormField>,
        output_path: String,
    ) -> PdfResult<String> {
        let mut doc = Document::load(path).map_err(|e| PdfError::OpenFailed(e.to_string()))?;

        if let Ok(catalog) = doc.catalog_mut() {
            let acro_ref = catalog
                .get(b"AcroForm")
                .ok()
                .and_then(|o| o.as_reference().ok());
            let _ = catalog;
            if let Some(acro_ref) = acro_ref {
                if let Some(Object::Dictionary(acro)) = doc.objects.get_mut(&acro_ref) {
                    if let Ok(fields_obj) = acro.get(b"Fields") {
                        if let Ok(fields_arr) = fields_obj.as_array() {
                            let fields_refs: Vec<ObjectId> = fields_arr
                                .iter()
                                .filter_map(|f| f.as_reference().ok())
                                .collect();
                            for r in fields_refs {
                                Self::walk_lopdf_fields(&mut doc, r, "", &updates);
                            }
                        }
                    }
                }
            }
        }

        doc.save(&output_path)
            .map_err(|e| PdfError::IoError(e.to_string()))?;

        Ok(output_path)
    }

    #[cfg(windows)]
    pub fn print_document(path: &str, printer_name: Option<&str>) -> PdfResult<()> {
        use winprint::printer::{FilePrinter, PrinterDevice, WinPdfPrinter};

        let all_devices = PrinterDevice::all()
            .map_err(|e| PdfError::IoError(format!("Failed to list printers: {e}")))?;

        let device = if let Some(name) = printer_name {
            all_devices
                .into_iter()
                .find(|d| d.name() == name)
                .ok_or_else(|| PdfError::IoError(format!("Printer '{name}' not found")))?
        } else {
            all_devices
                .into_iter()
                .next()
                .ok_or_else(|| PdfError::IoError("No printers found".into()))?
        };

        let printer = WinPdfPrinter::new(device);
        printer
            .print(std::path::Path::new(path), Default::default())
            .map_err(|e| PdfError::IoError(format!("Print failed: {e}")))?;
        Ok(())
    }

    #[cfg(not(windows))]
    pub fn print_document(_path: &str, _printer_name: Option<&str>) -> PdfResult<()> {
        Err(PdfError::IoError(
            "Printing is only supported on Windows".into(),
        ))
    }

    /// Returns a sorted list of all available printer names on this system.
    #[cfg(windows)]
    pub fn list_printers() -> PdfResult<Vec<String>> {
        use winprint::printer::PrinterDevice;
        PrinterDevice::all()
            .map(|devices| {
                let mut names: Vec<String> =
                    devices.into_iter().map(|d| d.name().to_string()).collect();
                names.sort_unstable();
                names
            })
            .map_err(|e| PdfError::IoError(format!("Failed to list printers: {e}")))
    }

    #[cfg(not(windows))]
    pub fn list_printers() -> PdfResult<Vec<String>> {
        Ok(Vec::new())
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

        let mut content = pdf_writer::Content::new();
        content.begin_text();
        content.set_font(pdf_writer::Name(b"F1"), 48.0);
        content.set_fill_rgb(0.7, 0.7, 0.7);
        content.set_text_matrix([1.0, 0.0, 0.0, 1.0, 200.0, 400.0]);
        content.show(pdf_writer::Str(text.as_bytes()));
        content.end_text();
        let watermark_stream =
            lopdf::Stream::new(lopdf::Dictionary::new(), content.finish().to_vec());

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
            auto_crop: false,
            quality: RenderQuality::Medium,
        };
        let key2 = RenderKey {
            doc_id,
            page_num: 0,
            scale: 100,
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
            auto_crop: false,
            quality: RenderQuality::Medium,
        };
        let key2 = RenderKey {
            doc_id,
            page_num: 1,
            scale: 100,
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
            auto_crop: false,
            quality: RenderQuality::Medium,
        };
        let key2 = RenderKey {
            doc_id,
            page_num: 0,
            scale: 200,
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
            auto_crop: false,
            quality: RenderQuality::Medium,
        };
        let key2 = RenderKey {
            doc_id: DocumentId(2),
            page_num: 0,
            scale: 100,
            auto_crop: false,
            quality: RenderQuality::Medium,
        };
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_render_key_distinguishes_scale() {
        let doc_id = DocumentId(1);
        let key_low = RenderKey {
            doc_id,
            page_num: 0,
            scale: 100,
            auto_crop: false,
            quality: RenderQuality::Medium,
        };
        let key_high = RenderKey {
            doc_id,
            page_num: 0,
            scale: 200,
            auto_crop: false,
            quality: RenderQuality::Medium,
        };
        assert_ne!(key_low, key_high);
    }

    #[test]
    fn test_render_cache_creation() {
        let cache = RenderCache::new(10, 100);
        assert_eq!(
            cache.get(&RenderKey {
                doc_id: DocumentId(1),
                page_num: 0,
                scale: 100,
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
            auto_crop: false,
            quality: RenderQuality::Medium,
        };
        let result = crate::models::RenderResult {
            width: 100,
            height: 100,
            data: vec![0u8; 100].into(),
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
            auto_crop: false,
            quality: RenderQuality::Medium,
        };
        let result1 = crate::models::RenderResult {
            width: 100,
            height: 100,
            data: vec![0u8; 100].into(),
        };
        let result2 = crate::models::RenderResult {
            width: 200,
            height: 200,
            data: vec![0u8; 200].into(),
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
            auto_crop: false,
            quality: RenderQuality::Medium,
        };
        let key2 = RenderKey {
            doc_id: DocumentId(1),
            page_num: 1,
            scale: 100,
            auto_crop: false,
            quality: RenderQuality::Medium,
        };
        let result1 = crate::models::RenderResult {
            width: 100,
            height: 100,
            data: vec![0u8; 100].into(),
        };
        let result2 = crate::models::RenderResult {
            width: 200,
            height: 200,
            data: vec![0u8; 200].into(),
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
    fn test_apply_filter_grayscale() {
        let mut data = vec![100, 150, 200, 255];
        DocumentStore::apply_filter(&mut data, RenderFilter::Grayscale);
        let luma = ((100 * 299 + 150 * 587 + 200 * 114) / 1000) as u8;
        assert_eq!(data[0], luma);
        assert_eq!(data[1], luma);
        assert_eq!(data[2], luma);
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
        let data = vec![255u8; 400];
        let result = DocumentStore::detect_content_bbox_parallel(&data, 10, 10);
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_content_bbox_parallel_full() {
        let mut data = vec![255u8; 400];
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
        let (min_x, min_y, _max_x, _max_y) = result.unwrap();
        assert!(min_x <= 10);
        assert!(min_y <= 10);
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
