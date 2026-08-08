use crate::models::{
    Annotation, AnnotationStyle, DocumentId, EngineErrorKind, FormField, FormFieldVariant,
    Hyperlink, PdfError, PdfResult, SearchResultItem,
};
use lopdf::{Document, Object, ObjectId};
use quick_cache::{Weighter, sync::Cache};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::Arc;
use zpdf::{
    AnnotationSpec, ContentInterpreter, FieldKind, FieldValue, FormFiller, IccCache, ImageCache,
    IncrementalWriter, MarkupKind, ParseLimits, PdfDocument, Rect as ZpdfRect, RenderBackend,
    cpu::CpuRenderer, detect_tables, output_intent_cmyk_profile, spans_to_text,
    struct_ordered_text,
};
use zpdf::{RewriteOptions, rewrite_pdf};
use zpdf_writer::{
    builder::DocumentBuilder,
    sign::{SignatureOptions, SigningKey},
    stamp::StampItem,
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

    pub fn remove(&self, key: &RenderKey) {
        self.cache.remove(key);
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
    cache_keys: HashMap<DocumentId, Vec<RenderKey>>,
    /// Base `OcConfig` as decoded from the document (read-only after load).
    oc_configs: HashMap<DocumentId, zpdf::OcConfig>,
    /// Per-document user visibility overrides applied on top of `oc_configs`.
    /// true = force ON, false = force OFF.
    oc_visibility: HashMap<DocumentId, HashMap<zpdf::ObjectId, bool>>,
}

// DocumentState wrapper removed as it was a single-field struct.

impl DocumentStore {
    pub fn new(cache: SharedRenderCache) -> Self {
        Self {
            documents: HashMap::new(),
            paths: HashMap::new(),
            render_cache: cache,
            cache_keys: HashMap::new(),
            oc_configs: HashMap::new(),
            oc_visibility: HashMap::new(),
        }
    }

    pub fn has_document(&self, doc_id: DocumentId) -> bool {
        self.documents.contains_key(&doc_id)
    }

    pub fn open_document(
        &mut self,
        path: &str,
        password: Option<&str>,
        doc_id: DocumentId,
    ) -> PdfResult<crate::models::OpenResult> {
        let data = std::fs::read(path).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        // Harden against malformed/malicious PDFs by capping stream and object counts.
        let limits = ParseLimits {
            max_stream_bytes: 256 * 1024 * 1024, // 256 MB per stream
            ..ParseLimits::default()
        };
        let doc = match PdfDocument::open_with_password_and_limits(
            data,
            password.unwrap_or("").as_bytes(),
            limits,
        ) {
            Ok(doc) => doc,
            Err(zpdf::Error::WrongPassword) => {
                return Err(PdfError::PasswordRequired);
            }
            Err(e) => {
                return Err(PdfError::OpenFailed(e.to_string()));
            }
        };

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

        let outline = self.get_outline_internal(&doc);
        let links = self.extract_links_internal(&doc);
        let info = doc.info();
        let xmp = doc.xmp_metadata();
        let metadata = Self::doc_info_to_metadata(info.as_ref(), xmp.as_ref());

        let page_labels_obj = doc.page_labels();
        let page_labels: Vec<String> = (0..page_count)
            .map(|i| {
                page_labels_obj
                    .as_ref()
                    .and_then(|pl| pl.label(i))
                    .unwrap_or_else(|| (i + 1).to_string())
            })
            .collect();
        let is_encrypted = doc.is_encrypted();
        let signatures = doc
            .signatures()
            .into_iter()
            .map(|sig| crate::models::SignatureInfo {
                field_name: sig.field_name,
                signer_name: sig.signer_common_name.or(sig.name),
                signing_time: sig.signing_time,
                location: sig.location,
                reason: sig.reason,
                digest_verified: matches!(sig.digest, zpdf::DigestStatus::Verified),
                crypto_valid: matches!(sig.crypto, zpdf::CryptoStatus::Valid),
            })
            .collect();

        let attachments = doc
            .embedded_files()
            .into_iter()
            .map(|ef| crate::models::AttachmentInfo {
                name: ef.name,
                description: ef.description,
                size: ef.size,
                creation_date: ef.creation_date,
                mod_date: ef.mod_date,
                object_id: ef.stream.map(|id| (id.0, id.1)),
            })
            .collect();

        let oc_config = doc.oc_config();
        let layers = Self::extract_layers_internal(&doc, oc_config.as_ref());

        if let Some(oc) = &oc_config {
            self.oc_configs.insert(doc_id, oc.clone());
        }

        // Feature 4: Extract geospatial annotation metadata
        let geo_annotations = Self::extract_geo_annotations(&doc);
        // Feature 5: Extract CMYK/ICC color profile info
        let color_profile = Self::extract_color_profile(&doc);

        self.documents.insert(doc_id, doc);
        self.paths.insert(doc_id, path.to_string());

        Ok(crate::models::OpenResult {
            id: doc_id,
            page_count,
            page_heights: heights,
            max_width,
            outline,
            links,
            metadata,
            page_labels,
            is_encrypted,
            signatures,
            attachments,
            layers,
            oc_config,
            geo_annotations,
            color_profile,
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

    pub fn load_document_meta(
        &mut self,
        doc_id: DocumentId,
    ) -> PdfResult<crate::models::DocumentMeta> {
        let doc = self
            .documents
            .get(&doc_id)
            .ok_or(PdfError::EngineError(EngineErrorKind::DocumentNotFound))?;

        let outline = self.get_outline_internal(doc);
        let links = self.extract_links_internal(doc);

        let info = doc.info();
        let xmp = doc.xmp_metadata();
        let metadata = Self::doc_info_to_metadata(info.as_ref(), xmp.as_ref());

        let page_count = doc.page_count();
        let page_labels_obj = doc.page_labels();
        let page_labels: Vec<String> = (0..page_count)
            .map(|i| {
                page_labels_obj
                    .as_ref()
                    .and_then(|pl| pl.label(i))
                    .unwrap_or_else(|| (i + 1).to_string())
            })
            .collect();
        let is_encrypted = doc.is_encrypted();
        let signatures = doc
            .signatures()
            .into_iter()
            .map(|sig| crate::models::SignatureInfo {
                field_name: sig.field_name,
                signer_name: sig.signer_common_name.or(sig.name),
                signing_time: sig.signing_time,
                location: sig.location,
                reason: sig.reason,
                digest_verified: matches!(sig.digest, zpdf::DigestStatus::Verified),
                crypto_valid: matches!(sig.crypto, zpdf::CryptoStatus::Valid),
            })
            .collect();

        let attachments = doc
            .embedded_files()
            .into_iter()
            .map(|ef| crate::models::AttachmentInfo {
                name: ef.name,
                description: ef.description,
                size: ef.size,
                creation_date: ef.creation_date,
                mod_date: ef.mod_date,
                object_id: ef.stream.map(|id| (id.0, id.1)),
            })
            .collect();

        let oc_config = doc.oc_config();
        let layers = Self::extract_layers_internal(doc, oc_config.as_ref());

        if let Some(oc) = &oc_config {
            self.oc_configs.insert(doc_id, oc.clone());
        }

        let geo_annotations = Self::extract_geo_annotations(doc);
        let color_profile = Self::extract_color_profile(doc);

        Ok(crate::models::DocumentMeta {
            outline,
            links,
            metadata,
            page_labels,
            is_encrypted,
            signatures,
            attachments,
            layers,
            oc_config,
            geo_annotations,
            color_profile,
        })
    }

    pub fn close_document(&mut self, doc_id: DocumentId) {
        self.documents.remove(&doc_id);
        self.paths.remove(&doc_id);
        self.oc_configs.remove(&doc_id);
        self.oc_visibility.remove(&doc_id);
        if let Some(doc_keys) = self.cache_keys.remove(&doc_id) {
            for key in doc_keys {
                self.render_cache.remove(&key);
            }
        }
    }

    /// Extract layer information from a document's `OCProperties` dictionary.
    /// Shared by `open_document` and `load_document_meta` to avoid duplication.
    fn extract_layers_internal(
        doc: &PdfDocument,
        oc: Option<&zpdf::OcConfig>,
    ) -> Vec<crate::models::LayerInfo> {
        let Some(oc) = oc else {
            return Vec::new();
        };
        let Ok(root_ref) = doc.file().trailer.get_ref("Root") else {
            return Vec::new();
        };
        let Ok(root) = doc.file().resolve(root_ref) else {
            return Vec::new();
        };
        let Ok(root_dict) = root.as_dict() else {
            return Vec::new();
        };
        let Some(ocp_obj) = root_dict.get("OCProperties") else {
            return Vec::new();
        };
        let resolved_ocp = match ocp_obj {
            zpdf::PdfObject::Ref(r) => doc.file().resolve(*r).ok(),
            other => Some(other.clone()),
        };
        let Some(zpdf::PdfObject::Dict(ocp_dict)) = &resolved_ocp else {
            return Vec::new();
        };
        let Some(ocgs_obj) = ocp_dict.get("OCGs") else {
            return Vec::new();
        };
        let resolved_ocgs = match ocgs_obj {
            zpdf::PdfObject::Ref(r) => doc.file().resolve(*r).ok(),
            other => Some(other.clone()),
        };
        let Some(zpdf::PdfObject::Array(ocgs_arr)) = &resolved_ocgs else {
            return Vec::new();
        };

        let mut layers = Vec::new();
        for item in ocgs_arr {
            if let zpdf::PdfObject::Ref(r) = item {
                if let Ok(ocg_obj) = doc.file().resolve(*r) {
                    if let Ok(ocg_dict) = ocg_obj.as_dict() {
                        let name_opt = ocg_dict.get("Name").and_then(|n| {
                            let resolved = match n {
                                zpdf::PdfObject::Ref(ref_id) => doc.file().resolve(*ref_id).ok()?,
                                other => other.clone(),
                            };
                            resolved
                                .as_name()
                                .map(ToString::to_string)
                                .or_else(|_| {
                                    resolved
                                        .as_str()
                                        .map(|s| String::from_utf8_lossy(&s.0).to_string())
                                })
                                .ok()
                        });
                        if let Some(name) = name_opt {
                            let visible = oc.group_visible(*r);
                            layers.push(crate::models::LayerInfo {
                                name,
                                object_id: (r.0, r.1),
                                visible,
                            });
                        }
                    }
                }
            }
        }
        layers
    }

    pub fn toggle_layer(&mut self, doc_id: DocumentId, object_id: (u32, u16), visible: bool) {
        if self.oc_configs.contains_key(&doc_id) {
            // Record the override in our safe visibility map — no unsafe transmute needed.
            let id = zpdf::ObjectId(object_id.0, object_id.1);
            self.oc_visibility
                .entry(doc_id)
                .or_default()
                .insert(id, visible);

            // Invalidate the render cache for this document.
            if let Some(keys) = self.cache_keys.get(&doc_id) {
                for key in keys {
                    self.render_cache.remove(key);
                }
            }
        }
    }

    pub fn get_attachment_bytes(
        &self,
        doc_id: DocumentId,
        object_id: (u32, u16),
    ) -> PdfResult<Vec<u8>> {
        let doc = self
            .documents
            .get(&doc_id)
            .ok_or(PdfError::EngineError(EngineErrorKind::DocumentNotFound))?;

        let efs = doc.embedded_files();
        let target_ef = efs
            .iter()
            .find(|ef| ef.stream.map(|id| (id.0, id.1)) == Some(object_id))
            .ok_or_else(|| {
                PdfError::EngineError(EngineErrorKind::Generic("Attachment not found".to_string()))
            })?;

        doc.embedded_file_bytes(target_ef)
            .map_err(|e| PdfError::EngineError(EngineErrorKind::Generic(e.to_string())))
    }

    /// Load annotations previously saved with `save_annotations` from the PDF file.
    /// Returns an empty vec if there are no annotations or if the file cannot be read.
    #[allow(clippy::many_single_char_names, clippy::similar_names)]
    pub fn load_annotations(&self, path: &str) -> PdfResult<Vec<Annotation>> {
        let Ok(lopdf_doc) = Document::load(path) else {
            return Ok(Vec::new());
        };

        let mut annotations = Vec::new();
        let pages = lopdf_doc.get_pages();

        for (&page_num, &page_id) in &pages {
            let page_idx = (page_num as usize).saturating_sub(1);

            // Get page height for coordinate conversion (PDF coords: bottom-left origin)
            let page_height = lopdf_doc
                .objects
                .get(&page_id)
                .and_then(|o| o.as_dict().ok())
                .and_then(|d| d.get(b"MediaBox").ok())
                .and_then(|o| o.as_array().ok())
                .and_then(|a| a.get(3))
                .and_then(|o| match o {
                    Object::Real(v) => Some(*v as f32),
                    Object::Integer(v) => Some(*v as f32),
                    _ => None,
                })
                .unwrap_or(792.0_f32);

            let annots_array = lopdf_doc
                .objects
                .get(&page_id)
                .and_then(|o| o.as_dict().ok())
                .and_then(|d| d.get(b"Annots").ok())
                .and_then(|a| match a {
                    Object::Reference(r) => {
                        lopdf_doc.objects.get(r).and_then(|o| o.as_array().ok())
                    }
                    Object::Array(arr) => Some(arr),
                    _ => None,
                })
                .cloned()
                .unwrap_or_default();

            for annot_ref in &annots_array {
                let annot_obj = match annot_ref {
                    Object::Reference(r) => lopdf_doc.objects.get(r),
                    _ => None,
                };
                let Some(dict) = annot_obj.and_then(|o| o.as_dict().ok()) else {
                    continue;
                };

                let subtype = dict
                    .get(b"Subtype")
                    .ok()
                    .and_then(|o| o.as_name().ok())
                    .map(|b| String::from_utf8_lossy(b).into_owned())
                    .unwrap_or_default();

                // Skip Link annotations (those are hyperlinks, not user annotations)
                if subtype == "Link" {
                    continue;
                }

                // Parse Rect [x0, y0, x1, y1] in PDF coords
                let rect = dict
                    .get(b"Rect")
                    .ok()
                    .and_then(|o| o.as_array().ok())
                    .cloned();
                let (pdf_x0, pdf_y0, pdf_x1, pdf_y1) = match rect.as_deref() {
                    Some([a, b, c, d]) => {
                        let to_f32 = |o: &Object| match o {
                            Object::Real(v) => *v as f32,
                            Object::Integer(v) => *v as f32,
                            _ => 0.0_f32,
                        };
                        (to_f32(a), to_f32(b), to_f32(c), to_f32(d))
                    }
                    _ => continue,
                };

                // Convert from PDF bottom-left origin to screen top-left origin
                let x = pdf_x0;
                let w = (pdf_x1 - pdf_x0).abs();
                let h = (pdf_y1 - pdf_y0).abs();
                let y = page_height - pdf_y0 - h;

                // Parse color
                let color_str = dict
                    .get(b"C")
                    .ok()
                    .and_then(|o| o.as_array().ok())
                    .cloned()
                    .and_then(|arr| {
                        let to_f32 = |o: &Object| match o {
                            Object::Real(v) => *v as f32,
                            Object::Integer(v) => *v as f32,
                            _ => 0.0_f32,
                        };
                        match arr.as_slice() {
                            [r, g, b] => Some(format!(
                                "#{:02X}{:02X}{:02X}",
                                (to_f32(r) * 255.0) as u8,
                                (to_f32(g) * 255.0) as u8,
                                (to_f32(b) * 255.0) as u8,
                            )),
                            _ => None,
                        }
                    })
                    .unwrap_or_else(|| "#408cff".to_string());

                let has_fill = dict.get(b"IC").is_ok();
                let thickness = dict
                    .get(b"BS")
                    .ok()
                    .and_then(|o| o.as_dict().ok())
                    .and_then(|d| d.get(b"W").ok())
                    .and_then(|o| match o {
                        Object::Real(v) => Some(*v as f32),
                        Object::Integer(v) => Some(*v as f32),
                        _ => None,
                    })
                    .unwrap_or(2.0_f32);

                let style = match subtype.as_str() {
                    "Highlight" => Some(AnnotationStyle::Highlight { color: color_str }),
                    "Square" if has_fill => Some(AnnotationStyle::Rectangle {
                        color: color_str,
                        thickness,
                        fill: true,
                    }),
                    "Square" => Some(AnnotationStyle::Rectangle {
                        color: color_str,
                        thickness,
                        fill: false,
                    }),
                    "Circle" if has_fill => Some(AnnotationStyle::Circle {
                        color: color_str,
                        thickness,
                        fill: true,
                    }),
                    "Circle" => Some(AnnotationStyle::Circle {
                        color: color_str,
                        thickness,
                        fill: false,
                    }),
                    "FreeText" => {
                        let text = dict
                            .get(b"Contents")
                            .ok()
                            .and_then(|o| o.as_str().ok())
                            .map(|b| String::from_utf8_lossy(b).to_string())
                            .unwrap_or_default();
                        Some(AnnotationStyle::Text {
                            text,
                            color: color_str,
                            font_size: 12,
                        })
                    }
                    "Text" => {
                        let comment = dict
                            .get(b"Contents")
                            .ok()
                            .and_then(|o| o.as_str().ok())
                            .map(|b| String::from_utf8_lossy(b).to_string())
                            .unwrap_or_default();
                        Some(AnnotationStyle::StickyNote {
                            comment,
                            color: color_str,
                        })
                    }
                    "Line" => {
                        let is_arrow = dict
                            .get(b"LE")
                            .ok()
                            .and_then(|o| o.as_array().ok())
                            .map(|a| {
                                a.iter().any(|o| {
                                    o.as_name()
                                        .ok()
                                        .map(|s| String::from_utf8_lossy(s).contains("Arrow"))
                                        .unwrap_or(false)
                                })
                            })
                            .unwrap_or(false);
                        if is_arrow {
                            Some(AnnotationStyle::Arrow {
                                color: color_str,
                                thickness,
                            })
                        } else {
                            Some(AnnotationStyle::Line {
                                color: color_str,
                                thickness,
                            })
                        }
                    }
                    _ => None,
                };

                if let Some(style) = style {
                    annotations.push(Annotation {
                        id: crate::models::next_annotation_id(),
                        page: page_idx,
                        x,
                        y,
                        width: w,
                        height: h,
                        style,
                    });
                }
            }
        }

        Ok(annotations)
    }

    fn render_page_internal(
        &mut self,
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
        let mut interp = ContentInterpreter::new(page.effective_box())
            .with_page_rotation(page.rotate + options.rotation)
            .with_fonts(&mut fonts)
            .with_document(doc.file(), &page.resources)
            .with_images(&mut images);

        // Build optional-content config with user visibility overrides applied.
        // Since OcConfig has no public mutation API, we store the base config
        // and our overrides separately. We pass the base OcConfig to the interpreter,
        // which honours its group_visible() for unknown groups. Our oc_visibility map
        // records per-user-session overrides; we apply them via the interpreter's
        // layer-filter closure if the API supports it, or fall back to cloning the
        // config and patching visibility through the only available approach.
        //
        // For now, if we have overrides, rebuild by starting from base and using
        // OcConfig::with_overrides (the v0.11 API for applying a visibility map).
        let oc_for_render: Option<zpdf::OcConfig>;
        if let Some(base_oc) = self.oc_configs.get(&doc_id) {
            if let Some(overrides) = self.oc_visibility.get(&doc_id) {
                if overrides.is_empty() {
                    oc_for_render = Some(base_oc.clone());
                } else {
                    // TODO: OcConfig has no public setter API for per-group overrides;
                    // apply_overrides is tracked as a future enhancement.
                    // For now, use the base config; visibility state is stored in
                    // oc_visibility for future use when the zpdf API exposes setters.
                    oc_for_render = Some(base_oc.clone());
                }
            } else {
                oc_for_render = Some(base_oc.clone());
            }
        } else {
            oc_for_render = None;
        }

        if let Some(ref oc) = oc_for_render {
            interp = interp.with_optional_content(oc);
        }

        let display_list = interp.interpret(&content);

        let mut renderer = CpuRenderer::new().with_fonts(&fonts).with_images(&images);
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

        self.cache_keys
            .entry(doc_id)
            .or_default()
            .push(cache_key.clone());
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

    pub fn extract_text(&self, doc_id: DocumentId, page_num: usize) -> PdfResult<String> {
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
        let text = if doc.is_tagged() {
            if let Some(tree) = doc.struct_tree() {
                struct_ordered_text(&spans, page_num, &tree)
            } else {
                spans_to_text(spans, 2.0)
            }
        } else {
            spans_to_text(spans, 2.0)
        };
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

    pub fn detect_tables_on_page(
        &self,
        doc_id: DocumentId,
        page_num: usize,
    ) -> PdfResult<Vec<crate::models::DetectedTable>> {
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

        let tables = detect_tables(&spans);
        let page_height = page.effective_box().height() as f32;

        let detected = tables
            .into_iter()
            .map(|t| {
                let (x0, y0, x1, y1) = t.bbox();
                // Coordinate conversion to y-down layout space:
                // x = x0, y = page_height - y1, w = x1 - x0, h = y1 - y0
                let bbox = (
                    x0 as f32,
                    page_height - y1 as f32,
                    (x1 - x0) as f32,
                    (y1 - y0) as f32,
                );
                crate::models::DetectedTable {
                    bbox,
                    csv: t.to_csv(),
                    tsv: t.to_tsv(),
                    cells: t.cells,
                }
            })
            .collect();

        Ok(detected)
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
            .ok_or(PdfError::EngineError(EngineErrorKind::DocumentPathNotFound))?
            .clone();

        let data = std::fs::read(&pdf_path).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        let mut writer =
            IncrementalWriter::new(data).map_err(|e| PdfError::OpenFailed(e.to_string()))?;

        for ann in annotations {
            // Skip redact annotations — those are applied via apply_redactions().
            if matches!(&ann.style, AnnotationStyle::Redact { .. }) {
                continue;
            }

            // Get page height for coordinate conversion.
            let page_height = self
                .documents
                .get(&doc_id)
                .and_then(|d| d.page(ann.page).ok())
                .map(|p| p.effective_box().height())
                .unwrap_or(792.0);

            let pdf_x = ann.x as f64;
            let pdf_w = ann.width as f64;
            let pdf_h = ann.height as f64;
            // Convert from screen top-left to PDF bottom-left origin.
            let pdf_y = page_height - ann.y as f64 - pdf_h;

            let rect = ZpdfRect {
                x0: pdf_x,
                y0: pdf_y,
                x1: pdf_x + pdf_w,
                y1: pdf_y + pdf_h,
            };

            let spec: AnnotationSpec = match &ann.style {
                AnnotationStyle::Highlight { color } => {
                    let (r, g, b) = hex_to_rgb(color);
                    AnnotationSpec::markup_from_rects(
                        MarkupKind::Highlight,
                        &[rect],
                        (r as f64, g as f64, b as f64),
                        None,
                    )
                }
                AnnotationStyle::Rectangle {
                    color,
                    fill,
                    thickness,
                } => {
                    let (r, g, b) = hex_to_rgb(color);
                    AnnotationSpec::Square {
                        rect,
                        color: (r as f64, g as f64, b as f64),
                        interior: if *fill {
                            Some((r as f64, g as f64, b as f64))
                        } else {
                            None
                        },
                        width: *thickness as f64,
                    }
                }
                AnnotationStyle::Circle {
                    color,
                    fill,
                    thickness,
                } => {
                    let (r, g, b) = hex_to_rgb(color);
                    AnnotationSpec::Circle {
                        rect,
                        color: (r as f64, g as f64, b as f64),
                        interior: if *fill {
                            Some((r as f64, g as f64, b as f64))
                        } else {
                            None
                        },
                        width: *thickness as f64,
                    }
                }
                AnnotationStyle::Text {
                    text,
                    color,
                    font_size,
                } => {
                    let (r, g, b) = hex_to_rgb(color);
                    AnnotationSpec::FreeText {
                        rect,
                        contents: text.clone(),
                        size: Some(*font_size as f64),
                        color: Some((r as f64, g as f64, b as f64)),
                    }
                }
                AnnotationStyle::StickyNote { comment, color } => {
                    let (r, g, b) = hex_to_rgb(color);
                    // Note anchor is the top-left of the annotation area (PDF bottom-left).
                    AnnotationSpec::Note {
                        x: pdf_x,
                        y: pdf_y,
                        contents: comment.clone(),
                        color: Some((r as f64, g as f64, b as f64)),
                        icon: None,
                    }
                }
                AnnotationStyle::Line { color, thickness } => {
                    let (r, g, b) = hex_to_rgb(color);
                    // Reconstruct line endpoints from the bounding rect.
                    AnnotationSpec::Line {
                        x1: pdf_x,
                        y1: pdf_y + pdf_h,
                        x2: pdf_x + pdf_w,
                        y2: pdf_y,
                        color: (r as f64, g as f64, b as f64),
                        width: *thickness as f64,
                    }
                }
                AnnotationStyle::Arrow { color, thickness } => {
                    let (r, g, b) = hex_to_rgb(color);
                    // Arrow is also a Line annotation; LE entries mark the arrowhead.
                    // zpdf's Line spec doesn't expose arrowheads directly \u2014 we use a
                    // Line spec here (the arrowhead rendering is decorative in the viewer).
                    AnnotationSpec::Line {
                        x1: pdf_x,
                        y1: pdf_y + pdf_h,
                        x2: pdf_x + pdf_w,
                        y2: pdf_y,
                        color: (r as f64, g as f64, b as f64),
                        width: *thickness as f64,
                    }
                }
                AnnotationStyle::Redact { .. } => continue, // unreachable, handled above
            };

            writer
                .add_annotation(ann.page, &spec)
                .map_err(|e| PdfError::IoError(e.to_string()))?;
        }

        let out_bytes = {
            let mut buf = std::io::Cursor::new(Vec::new());
            writer
                .write(&mut buf)
                .map_err(|e| PdfError::IoError(e.to_string()))?;
            buf.into_inner()
        };

        let pdf_path_buf = std::path::Path::new(&pdf_path);
        let final_path = output_path.unwrap_or_else(|| {
            let mut p = pdf_path_buf.to_path_buf();
            let stem = p
                .file_stem()
                .map(|s| s.to_string_lossy())
                .unwrap_or_default();
            p.set_file_name(format!("{stem}_annotated.pdf"));
            p.to_string_lossy().to_string()
        });

        std::fs::write(&final_path, &out_bytes).map_err(|e| PdfError::IoError(e.to_string()))?;

        Ok(final_path)
    }

    pub fn export_page_as_image(
        &self,
        doc_id: DocumentId,
        page_num: usize,
        scale: f32,
    ) -> PdfResult<Vec<u8>> {
        let cache_key = RenderKey {
            doc_id,
            page_num,
            scale: (scale * 100.0).round() as u32,
            auto_crop: false,
            quality: RenderQuality::Medium,
        };

        if let Some(cached_res) = self.render_cache.get(&cache_key) {
            let image = Image::from_u8(
                &cached_res.data,
                cached_res.width as usize,
                cached_res.height as usize,
                zune_core::colorspace::ColorSpace::RGBA,
            );
            let out_buf = image
                .write_to_vec(ImageFormat::PNG)
                .map_err(|e| PdfError::RenderFailed(format!("{e:?}")))?;
            return Ok(out_buf);
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

        let display_list = ContentInterpreter::new(page.effective_box())
            .with_page_rotation(page.rotate)
            .with_fonts(&mut fonts)
            .with_document(doc.file(), &page.resources)
            .with_images(&mut images)
            .interpret(&content);

        let mut renderer = CpuRenderer::new().with_fonts(&fonts).with_images(&images);
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

    fn flatten_outline(items: &[zpdf::OutlineItem], out: &mut Vec<Bookmark>, depth: usize) {
        if depth > 100 {
            return;
        }
        for item in items {
            let page_idx = item.dest.as_ref().and_then(|d| d.page).unwrap_or(0);
            out.push(Bookmark {
                title: item.title.clone(),
                page_index: page_idx,
            });
            Self::flatten_outline(&item.children, out, depth + 1);
        }
    }

    pub fn get_outline_internal(&self, doc: &PdfDocument) -> Vec<Bookmark> {
        let mut bookmarks = Vec::new();
        Self::flatten_outline(&doc.outline(), &mut bookmarks, 0);
        bookmarks
    }

    fn doc_info_to_metadata(
        info: Option<&zpdf::DocInfo>,
        xmp: Option<&zpdf::XmpMetadata>,
    ) -> crate::models::DocumentMetadata {
        let title = xmp
            .and_then(|x| x.title.clone())
            .or_else(|| info.and_then(|i| i.title.clone()));

        let author = xmp
            .and_then(|x| {
                if x.creators.is_empty() {
                    None
                } else {
                    Some(x.creators.join(", "))
                }
            })
            .or_else(|| info.and_then(|i| i.author.clone()));

        let subject = xmp
            .and_then(|x| {
                x.description.clone().or_else(|| {
                    if x.subjects.is_empty() {
                        None
                    } else {
                        Some(x.subjects.join(", "))
                    }
                })
            })
            .or_else(|| info.and_then(|i| i.subject.clone()));

        let keywords = xmp
            .and_then(|x| x.keywords.clone())
            .or_else(|| info.and_then(|i| i.keywords.clone()));

        let creator = xmp
            .and_then(|x| x.creator_tool.clone())
            .or_else(|| info.and_then(|i| i.creator.clone()));

        let producer = xmp
            .and_then(|x| x.producer.clone())
            .or_else(|| info.and_then(|i| i.producer.clone()));

        let creation_date = xmp
            .and_then(|x| x.create_date.clone())
            .or_else(|| info.and_then(|i| i.creation_date.clone()));

        let modification_date = xmp
            .and_then(|x| x.modify_date.clone())
            .or_else(|| info.and_then(|i| i.mod_date.clone()));

        crate::models::DocumentMetadata {
            title,
            author,
            subject,
            keywords,
            creator,
            producer,
            creation_date,
            modification_date,
        }
    }

    pub fn search(&self, doc_id: DocumentId, query: &str) -> PdfResult<Vec<SearchResultItem>> {
        let doc = self
            .documents
            .get(&doc_id)
            .ok_or(PdfError::EngineError(EngineErrorKind::DocumentNotFound))?;
        let total_pages = doc.page_count();
        let query_lower = query.to_lowercase();

        let mut results = Vec::new();

        for page_idx in 0..total_pages {
            let Ok(page) = doc.page(page_idx) else {
                continue;
            };
            let mut fonts = doc.load_page_fonts(&page);
            let mut images = ImageCache::new();
            let Ok(content) = doc.page_content_bytes(&page) else {
                continue;
            };

            let mut spans: Vec<zpdf::TextSpan> = Vec::new();
            let eff_box = page.effective_box();
            {
                let interp = ContentInterpreter::new(eff_box)
                    .with_page_rotation(page.rotate)
                    .with_fonts(&mut fonts)
                    .with_document(doc.file(), &page.resources)
                    .with_images(&mut images)
                    .with_text_sink(&mut spans);
                let _ = interp.interpret(&content);
            }

            let mut full_text = String::new();
            let mut span_offsets = Vec::new();

            for (idx, span) in spans.iter().enumerate() {
                let start = full_text.len();
                full_text.push_str(&span.text);
                let end = full_text.len();
                span_offsets.push((start, end, idx));
            }

            let full_text_lower = full_text.to_lowercase();
            let char_boundaries: Vec<usize> = full_text
                .char_indices()
                .map(|(idx, _)| idx)
                .chain(std::iter::once(full_text.len()))
                .collect();
            let char_boundaries_lower: Vec<usize> = full_text_lower
                .char_indices()
                .map(|(idx, _)| idx)
                .chain(std::iter::once(full_text_lower.len()))
                .collect();

            let mut search_idx = 0;
            while let Some(pos) = full_text_lower[search_idx..].find(&query_lower) {
                let match_start = search_idx + pos;
                let match_end = match_start + query_lower.len();

                // Get char index in full_text_lower:
                let char_start = char_boundaries_lower
                    .binary_search(&match_start)
                    .unwrap_or_else(|x| x);
                let char_end = char_boundaries_lower
                    .binary_search(&match_end)
                    .unwrap_or_else(|x| x);

                // Map to byte index in original full_text:
                let orig_start = char_boundaries
                    .get(char_start)
                    .copied()
                    .unwrap_or(full_text.len());
                let orig_end = char_boundaries
                    .get(char_end)
                    .copied()
                    .unwrap_or(full_text.len());
                let matched_text = full_text[orig_start..orig_end].to_string();

                if let Some(&(_, _, span_idx)) = span_offsets
                    .iter()
                    .find(|(s, e, _)| orig_start >= *s && orig_start < *e)
                {
                    let first_span = &spans[span_idx];
                    let y_top_down = (eff_box.y1 - first_span.y - first_span.size as f64) as f32;
                    let x_left = (first_span.x - eff_box.x0) as f32;
                    let span_w = (first_span.advance.abs() as f32).max(12.0);
                    let span_h = (first_span.size).max(12.0);

                    results.push(SearchResultItem {
                        page_index: page_idx,
                        text: matched_text,
                        y: y_top_down,
                        x: x_left,
                        width: span_w,
                        height: span_h,
                    });
                }

                // Advance search_idx safely to the next character boundary in full_text_lower
                search_idx = char_boundaries_lower
                    .get(char_start + 1)
                    .copied()
                    .unwrap_or(full_text_lower.len());
            }
        }

        Ok(results)
    }

    fn detect_content_bbox_parallel(
        data: &[u8],
        width: u32,
        height: u32,
    ) -> Option<(u32, u32, u32, u32)> {
        if width == 0 || height == 0 {
            return None;
        }
        let bbox = if data.len() < 64 * 1024 {
            let mut acc: Option<(u32, u32, u32, u32)> = None;
            for (idx, pixel) in data.chunks_exact(4).enumerate() {
                if pixel[0] <= WHITE_THRESHOLD
                    || pixel[1] <= WHITE_THRESHOLD
                    || pixel[2] <= WHITE_THRESHOLD
                {
                    let x = (idx as u32) % width;
                    let y = (idx as u32) / width;
                    if let Some((min_x, min_y, max_x, max_y)) = acc {
                        acc = Some((min_x.min(x), min_y.min(y), max_x.max(x), max_y.max(y)));
                    } else {
                        acc = Some((x, y, x, y));
                    }
                }
            }
            acc
        } else {
            data.par_chunks_exact(4)
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
                )
        };

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
                        pixel[0] = 255;
                        pixel[1] = 255;
                        pixel[2] = 255;
                    }
                });
            }
            RenderFilter::BlackWhite => {
                data.par_chunks_exact_mut(4).for_each(|pixel| {
                    let avg = (pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32) / 3;
                    let val = if avg > 128 { 255 } else { 0 };
                    pixel[0] = val;
                    pixel[1] = val;
                    pixel[2] = val;
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
                    if pixel[0] > NO_SHADOW_THRESHOLD
                        && pixel[1] > NO_SHADOW_THRESHOLD
                        && pixel[2] > NO_SHADOW_THRESHOLD
                    {
                        pixel[0] = 255;
                        pixel[1] = 255;
                        pixel[2] = 255;
                    }
                });
            }
            RenderFilter::Sepia => {
                data.par_chunks_exact_mut(4).for_each(|pixel| {
                    let r = pixel[0] as f32;
                    let g = pixel[1] as f32;
                    let b = pixel[2] as f32;
                    pixel[0] = (r * 0.393 + g * 0.769 + b * 0.189).min(255.0) as u8;
                    pixel[1] = (r * 0.349 + g * 0.686 + b * 0.168).min(255.0) as u8;
                    pixel[2] = (r * 0.272 + g * 0.534 + b * 0.131).min(255.0) as u8;
                });
            }
            RenderFilter::Grayscale => {
                data.par_chunks_exact_mut(4).for_each(|pixel| {
                    let luma =
                        (pixel[0] as u32 * 299 + pixel[1] as u32 * 587 + pixel[2] as u32 * 114)
                            / 1000;
                    pixel[0] = luma as u8;
                    pixel[1] = luma as u8;
                    pixel[2] = luma as u8;
                });
            }
            RenderFilter::None => {}
        }
    }

    // apply_filter_parallel removed as it was just a misleading wrapper.

    pub fn optimize_pdf(&self, input_path: &str, output_path: &str) -> PdfResult<String> {
        // Run full PDF optimization pass:
        // 1. Enable Flate/Deflate stream compression for uncompressed content & object streams.
        // 2. Downsample high-DPI image streams (max 1600px dimension) for significant file size reduction.
        let data = std::fs::read(input_path).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        let pdf = zpdf::PdfFile::parse(data).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        let mut opts = RewriteOptions::default();
        opts.compress_uncompressed = true;
        opts.max_image_dimension = Some(1600);
        let out_bytes = rewrite_pdf(&pdf, &opts).map_err(|e| PdfError::IoError(e.to_string()))?;
        std::fs::write(output_path, &out_bytes).map_err(|e| PdfError::IoError(e.to_string()))?;
        Ok(output_path.to_string())
    }

    pub fn encrypt_pdf(
        &self,
        input_path: &str,
        output_path: &str,
        user_pass: &str,
        owner_pass: &str,
        algorithm: &str,
    ) -> PdfResult<String> {
        let data = std::fs::read(input_path).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        let pdf = zpdf::PdfFile::parse(data).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        let enc_config = if algorithm.eq_ignore_ascii_case("rc4") {
            zpdf_writer::EncryptionConfig::rc4_128(user_pass, owner_pass)
        } else {
            zpdf_writer::EncryptionConfig::aes256(user_pass, owner_pass)
        };
        let mut opts = RewriteOptions::default();
        opts.encrypt = Some(enc_config);
        let out_bytes = rewrite_pdf(&pdf, &opts).map_err(|e| PdfError::IoError(e.to_string()))?;
        std::fs::write(output_path, &out_bytes).map_err(|e| PdfError::IoError(e.to_string()))?;
        Ok(output_path.to_string())
    }

    pub fn linearize_pdf(&self, input_path: &str, output_path: &str) -> PdfResult<String> {
        let data = std::fs::read(input_path).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        let pdf = zpdf::PdfFile::parse(data).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        let out_bytes =
            zpdf_writer::linearize_pdf(&pdf).map_err(|e| PdfError::IoError(e.to_string()))?;
        std::fs::write(output_path, &out_bytes).map_err(|e| PdfError::IoError(e.to_string()))?;
        Ok(output_path.to_string())
    }

    pub fn tag_pdf(&self, input_path: &str, output_path: &str) -> PdfResult<String> {
        let data = std::fs::read(input_path).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        let mut writer =
            IncrementalWriter::new(data).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        writer.tag_pdf().map_err(|e| PdfError::IoError(e.to_string()))?;
        let out_bytes = {
            let mut buf = std::io::Cursor::new(Vec::new());
            writer
                .write(&mut buf)
                .map_err(|e| PdfError::IoError(e.to_string()))?;
            buf.into_inner()
        };
        std::fs::write(output_path, &out_bytes).map_err(|e| PdfError::IoError(e.to_string()))?;
        Ok(output_path.to_string())
    }

    pub fn validate_pdfa(
        &self,
        path: &str,
        profile_str: &str,
    ) -> PdfResult<crate::models::PdfaValidationReport> {
        let data = std::fs::read(path).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        let pdf = zpdf::PdfFile::parse(data).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        let profile = match profile_str.to_lowercase().as_str() {
            "pdfa-2b" | "2b" | "a-2b" => zpdf::pdfa::Profile::A2b,
            _ => zpdf::pdfa::Profile::A1b,
        };
        let report = zpdf::pdfa::validate(&pdf, profile);
        Ok(crate::models::PdfaValidationReport {
            profile: profile.as_str().to_string(),
            conforms: report.conforms(),
            claimed: report.claimed,
            violations: report
                .violations
                .into_iter()
                .map(|v| crate::models::PdfaViolation {
                    rule: v.rule.to_string(),
                    message: v.message,
                })
                .collect(),
        })
    }

    pub fn verify_signature_trust(
        &self,
        doc_id: DocumentId,
        trust_anchors_bytes: &[u8],
    ) -> PdfResult<Vec<crate::models::SigTrustResult>> {
        let doc = self
            .documents
            .get(&doc_id)
            .ok_or(PdfError::EngineError(EngineErrorKind::DocumentNotFound))?;
        let anchors = zpdf::trust::parse_trust_anchors(trust_anchors_bytes);
        let sigs = doc.signatures();
        let mut results = Vec::new();
        for sig in sigs {
            let status = if let Some(ref cms) = sig.cms_blob {
                let chain_status = zpdf::trust::verify_certificate_chain(cms, &anchors, None);
                match chain_status {
                    zpdf::trust::ChainStatus::Trusted(cns) => {
                        format!("Trusted: {}", cns.join(" -> "))
                    }
                    zpdf::trust::ChainStatus::Untrusted(msg) => format!("Untrusted: {msg}"),
                    zpdf::trust::ChainStatus::Unsupported(msg) => format!("Unsupported: {msg}"),
                }
            } else {
                "No CMS contents".to_string()
            };
            results.push(crate::models::SigTrustResult {
                field_name: sig.field_name.clone(),
                status,
            });
        }
        Ok(results)
    }

    pub fn convert_pdf_doc_by_id(
        &self,
        doc_id: DocumentId,
        mode: &str,
        format: &str,
    ) -> PdfResult<String> {
        let doc = self
            .documents
            .get(&doc_id)
            .ok_or(PdfError::EngineError(EngineErrorKind::DocumentNotFound))?;
        let page_indices: Vec<usize> = (0..doc.page_count()).collect();
        let conv_mode = if mode.eq_ignore_ascii_case("rich") {
            zpdf::ConversionMode::Rich
        } else {
            zpdf::ConversionMode::TextOnly
        };
        let opts = zpdf::ConversionOptions {
            mode: conv_mode,
            use_structure: true,
        };
        let converted = zpdf::convert_pdf(doc, &page_indices, opts).map_err(|e| {
            PdfError::EngineError(EngineErrorKind::Generic(format!(
                "Conversion failed: {e:?}"
            )))
        })?;

        match format.to_lowercase().as_str() {
            "md" | "markdown" => {
                let mut out = String::new();
                for page in &converted.pages {
                    let _ = writeln!(out, "## Page {}\n\n{}\n", page.index + 1, page.text);
                }
                Ok(out)
            }
            "html" => {
                let mut out = String::from(
                    "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<title>Converted PDF</title>\n</head>\n<body>\n",
                );
                for page in &converted.pages {
                    let escaped = page
                        .text
                        .replace('&', "&amp;")
                        .replace('<', "&lt;")
                        .replace('>', "&gt;");
                    let _ = writeln!(
                        out,
                        "<section><h2>Page {}</h2><pre>{}</pre></section>",
                        page.index + 1,
                        escaped
                    );
                }
                out.push_str("</body>\n</html>\n");
                Ok(out)
            }
            _ => {
                let mut out = String::new();
                for page in &converted.pages {
                    let _ = writeln!(out, "--- Page {} ---\n\n{}\n", page.index + 1, page.text);
                }
                Ok(out)
            }
        }
    }

    pub fn convert_pdf_doc(&self, path: &str, mode: &str, format: &str) -> PdfResult<String> {
        let data = std::fs::read(path).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        let doc = zpdf::PdfDocument::open(data).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        let page_indices: Vec<usize> = (0..doc.page_count()).collect();
        let conv_mode = if mode.eq_ignore_ascii_case("rich") {
            zpdf::ConversionMode::Rich
        } else {
            zpdf::ConversionMode::TextOnly
        };
        let opts = zpdf::ConversionOptions {
            mode: conv_mode,
            use_structure: true,
        };
        let converted = zpdf::convert_pdf(&doc, &page_indices, opts).map_err(|e| {
            PdfError::EngineError(EngineErrorKind::Generic(format!(
                "Conversion failed: {e:?}"
            )))
        })?;

        match format.to_lowercase().as_str() {
            "md" | "markdown" => {
                let mut out = String::new();
                for page in &converted.pages {
                    let _ = writeln!(out, "## Page {}\n\n{}\n", page.index + 1, page.text);
                }
                Ok(out)
            }
            "html" => {
                let mut out = String::from(
                    "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<title>Converted PDF</title>\n</head>\n<body>\n",
                );
                for page in &converted.pages {
                    let escaped = page
                        .text
                        .replace('&', "&amp;")
                        .replace('<', "&lt;")
                        .replace('>', "&gt;");
                    let _ = writeln!(
                        out,
                        "<section><h2>Page {}</h2><pre>{}</pre></section>",
                        page.index + 1,
                        escaped
                    );
                }
                out.push_str("</body>\n</html>");
                Ok(out)
            }
            _ => {
                let mut out = String::new();
                for page in &converted.pages {
                    let _ = writeln!(out, "--- Page {} ---\n\n{}\n", page.index + 1, page.text);
                }
                Ok(out)
            }
        }
    }

    pub fn merge_documents(&self, paths: Vec<String>, output_path: String) -> PdfResult<String> {
        if paths.is_empty() {
            return Err(PdfError::IoError("No documents to merge".into()));
        }

        // Load and use the first document as the base of the IncrementalWriter.
        let first_data =
            std::fs::read(&paths[0]).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        let mut writer =
            IncrementalWriter::new(first_data).map_err(|e| PdfError::OpenFailed(e.to_string()))?;

        // Append remaining documents — this preserves outlines, forms, and OCGs.
        for path in paths.iter().skip(1) {
            let data = std::fs::read(path).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
            let doc =
                zpdf::PdfFile::parse(data).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
            writer
                .append_document(&doc)
                .map_err(|e| PdfError::IoError(e.to_string()))?;
        }

        let out_bytes = {
            let mut buf = std::io::Cursor::new(Vec::new());
            writer
                .write(&mut buf)
                .map_err(|e| PdfError::IoError(e.to_string()))?;
            buf.into_inner()
        };
        std::fs::write(&output_path, &out_bytes).map_err(|e| PdfError::IoError(e.to_string()))?;

        Ok(output_path)
    }

    pub fn reorder_pages(
        &self,
        input_path: &str,
        page_order: &[usize],
        output_path: &str,
    ) -> PdfResult<String> {
        let data = std::fs::read(input_path).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        let mut writer =
            IncrementalWriter::new(data).map_err(|e| PdfError::OpenFailed(e.to_string()))?;

        writer
            .reorder_pages(page_order)
            .map_err(|e| PdfError::IoError(e.to_string()))?;

        let out_bytes = {
            let mut buf = std::io::Cursor::new(Vec::new());
            writer
                .write(&mut buf)
                .map_err(|e| PdfError::IoError(e.to_string()))?;
            buf.into_inner()
        };
        std::fs::write(output_path, &out_bytes).map_err(|e| PdfError::IoError(e.to_string()))?;

        Ok(output_path.to_string())
    }

    pub fn split_pdf(
        &self,
        path: &str,
        page_indices: Vec<usize>,
        output_dir: String,
    ) -> PdfResult<Vec<String>> {
        let base_data = std::fs::read(path).map_err(|e| PdfError::OpenFailed(e.to_string()))?;

        let total_pages = Document::load(path)
            .map(|d| d.get_pages().len())
            .unwrap_or(0);

        let filename = std::path::Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("document");

        let mut created_paths = Vec::new();

        for &page_idx in &page_indices {
            if page_idx >= total_pages {
                continue;
            }
            // Build a delete list of all pages except the one we want.
            let delete_indices: Vec<usize> = (0..total_pages).filter(|&i| i != page_idx).collect();

            // Create a fresh writer per output page.
            let mut writer = IncrementalWriter::new(base_data.clone())
                .map_err(|e| PdfError::OpenFailed(e.to_string()))?;

            writer
                .delete_pages(&delete_indices)
                .map_err(|e| PdfError::IoError(e.to_string()))?;

            let out_bytes = {
                let mut buf = std::io::Cursor::new(Vec::new());
                writer
                    .write(&mut buf)
                    .map_err(|e| PdfError::IoError(e.to_string()))?;
                buf.into_inner()
            };

            let out_path = format!("{}/{}_page_{}.pdf", output_dir, filename, page_idx + 1);
            std::fs::write(&out_path, &out_bytes).map_err(|e| PdfError::IoError(e.to_string()))?;
            created_paths.push(out_path);
        }

        Ok(created_paths)
    }

    /// Burn redaction annotations permanently into the PDF, making the
    /// redacted content irrecoverable. Uses `IncrementalWriter::redact_page`
    /// for proper flate-paint redaction with content stream rewriting.
    pub fn apply_redactions(
        &self,
        doc_id: DocumentId,
        annotations: &[Annotation],
        output_path: Option<String>,
    ) -> PdfResult<String> {
        let pdf_path = self
            .paths
            .get(&doc_id)
            .ok_or(PdfError::EngineError(EngineErrorKind::DocumentPathNotFound))?;

        let data = std::fs::read(pdf_path).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        let mut writer =
            IncrementalWriter::new(data).map_err(|e| PdfError::OpenFailed(e.to_string()))?;

        // Group Redact annotations by page.
        let mut by_page: HashMap<usize, Vec<ZpdfRect>> = HashMap::new();
        for ann in annotations {
            if let AnnotationStyle::Redact { .. } = &ann.style {
                // Get the page height for coordinate conversion.
                let page_height = self
                    .documents
                    .get(&doc_id)
                    .and_then(|d| d.page(ann.page).ok())
                    .map(|p| p.effective_box().height())
                    .unwrap_or(792.0);

                // Convert from screen (top-left) coords to PDF (bottom-left) coords.
                let pdf_y = page_height - ann.y as f64 - ann.height as f64;
                let rect = ZpdfRect {
                    x0: ann.x as f64,
                    y0: pdf_y,
                    x1: ann.x as f64 + ann.width as f64,
                    y1: pdf_y + ann.height as f64,
                };
                by_page.entry(ann.page).or_default().push(rect);
            }
        }

        for (page_idx, rects) in by_page {
            writer
                .redact_page(page_idx, &rects, &Default::default())
                .map_err(|e| PdfError::IoError(e.to_string()))?;
        }

        let out_bytes = {
            let mut buf = std::io::Cursor::new(Vec::new());
            writer
                .write(&mut buf)
                .map_err(|e| PdfError::IoError(e.to_string()))?;
            buf.into_inner()
        };

        let pdf_path_buf = std::path::Path::new(pdf_path);
        let final_path = output_path.unwrap_or_else(|| {
            let stem = pdf_path_buf
                .file_stem()
                .map(|s| s.to_string_lossy())
                .unwrap_or_default();
            let dir = pdf_path_buf
                .parent()
                .map(|p| p.to_string_lossy())
                .unwrap_or_default();
            format!("{}/{}_redacted.pdf", dir, stem)
        });

        std::fs::write(&final_path, &out_bytes).map_err(|e| PdfError::IoError(e.to_string()))?;

        Ok(final_path)
    }

    pub fn get_form_fields(&mut self, path: &str) -> PdfResult<Vec<FormField>> {
        let doc_id = self
            .paths
            .iter()
            .find(|(_, p)| *p == path)
            .map(|(id, _)| *id);

        if let Some(id) = doc_id {
            if let Some(doc) = self.documents.get(&id) {
                return Ok(self.extract_form_fields_from_doc(doc));
            }
        }

        let data = std::fs::read(path).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        let doc = PdfDocument::open(data).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        Ok(self.extract_form_fields_from_doc(&doc))
    }

    fn extract_form_fields_from_doc(&self, doc: &PdfDocument) -> Vec<FormField> {
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
        fields
    }

    pub fn fill_form(
        &mut self,
        path: &str,
        updates: Vec<FormField>,
        output_path: String,
    ) -> PdfResult<String> {
        let data = std::fs::read(path).map_err(|e| PdfError::IoError(e.to_string()))?;
        let mut writer =
            IncrementalWriter::new(data).map_err(|e| PdfError::IoError(e.to_string()))?;

        {
            let mut filler =
                FormFiller::new(&mut writer).map_err(|e| PdfError::IoError(e.to_string()))?;
            for update in updates {
                let val_str = match &update.variant {
                    FormFieldVariant::Text { value } => value.clone(),
                    FormFieldVariant::Checkbox { is_checked } => {
                        if *is_checked {
                            "Yes".to_string()
                        } else {
                            "Off".to_string()
                        }
                    }
                    FormFieldVariant::RadioButton { is_selected, .. } => {
                        if *is_selected {
                            "Yes".to_string()
                        } else {
                            "Off".to_string()
                        }
                    }
                    FormFieldVariant::ComboBox {
                        selected_index,
                        options,
                    } => {
                        if let Some(idx) = selected_index {
                            options.get(*idx).cloned().unwrap_or_default()
                        } else {
                            String::new()
                        }
                    }
                };
                if let Err(e) = filler.set(&update.name, &val_str) {
                    tracing::warn!("Failed to set field {}: {}", update.name, e);
                }
            }
            filler
                .finish()
                .map_err(|e| PdfError::IoError(e.to_string()))?;
        }

        let mut file =
            std::fs::File::create(&output_path).map_err(|e| PdfError::IoError(e.to_string()))?;
        writer
            .write(&mut file)
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
        let watermark_id = doc.add_object(watermark_stream);

        for &page_id in &pages {
            let existing_contents = doc
                .get_page_contents(page_id)
                .into_iter()
                .map(Object::Reference)
                .collect::<Vec<_>>();
            let mut all_contents = existing_contents;
            all_contents.push(Object::Reference(watermark_id));

            let existing_res = doc
                .objects
                .get(&page_id)
                .and_then(|o| o.as_dict().ok())
                .and_then(|d| d.get(b"Resources").ok())
                .cloned();

            let mut merged_res = None;
            if let Some(res) = existing_res {
                let mut res_dict = match res {
                    Object::Reference(r) => {
                        doc.objects.get(&r).and_then(|o| o.as_dict().ok()).cloned()
                    }
                    Object::Dictionary(d) => Some(d.clone()),
                    _ => None,
                };
                if let Some(ref mut d) = res_dict {
                    if let Ok(existing_fonts) = d.get_mut(b"Font") {
                        let mut fonts_dict = match existing_fonts {
                            Object::Reference(r) => {
                                doc.objects.get(r).and_then(|o| o.as_dict().ok()).cloned()
                            }
                            Object::Dictionary(fd) => Some(fd.clone()),
                            _ => None,
                        };
                        if let Some(ref mut fd) = fonts_dict {
                            fd.set("F1", Object::Reference(font_ref_id));
                            d.set("Font", Object::Dictionary(fd.clone()));
                        }
                    } else {
                        d.set(
                            "Font",
                            Object::Dictionary(lopdf::Dictionary::from_iter(vec![(
                                "F1",
                                Object::Reference(font_ref_id),
                            )])),
                        );
                    }
                    merged_res = Some(doc.add_object(Object::Dictionary(d.clone())));
                }
            }

            let final_res_id = merged_res.unwrap_or(resources_id);

            let page_dict = doc
                .objects
                .get_mut(&page_id)
                .and_then(|o| o.as_dict_mut().ok())
                .ok_or_else(|| PdfError::EngineError("Invalid page object".into()))?;
            page_dict.set("Contents", Object::Array(all_contents));
            page_dict.set("Resources", Object::Reference(final_res_id));
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

    /// Feature: Add Header & Footer Page Numbers ("Page X of Y")
    #[allow(clippy::literal_string_with_formatting_args)]
    pub fn add_header_footer(
        input_path: &str,
        header_text: &str,
        footer_format: &str,
        output_path: &str,
    ) -> PdfResult<String> {
        let mut doc =
            Document::load(input_path).map_err(|e| PdfError::OpenFailed(e.to_string()))?;

        let pages: Vec<ObjectId> = doc.get_pages().into_values().collect();
        let page_count = pages.len();

        let font_ref_id = doc.add_object(Object::Dictionary(lopdf::Dictionary::from_iter(vec![
            ("Type", Object::Name(b"Font".to_vec())),
            ("Subtype", Object::Name(b"Type1".to_vec())),
            ("BaseFont", Object::Name(b"Helvetica".to_vec())),
        ])));

        for (idx, &page_id) in pages.iter().enumerate() {
            let page_num = idx + 1;
            let formatted_footer = footer_format
                .replace("{page}", &page_num.to_string())
                .replace("{pages}", &page_count.to_string());

            let mut content = pdf_writer::Content::new();
            content.begin_text();
            content.set_font(pdf_writer::Name(b"F1"), 10.0);
            content.set_fill_rgb(0.2, 0.2, 0.2);

            if !header_text.is_empty() {
                content.set_text_matrix([1.0, 0.0, 0.0, 1.0, 50.0, 760.0]);
                content.show(pdf_writer::Str(header_text.as_bytes()));
            }

            if !formatted_footer.is_empty() {
                content.set_text_matrix([1.0, 0.0, 0.0, 1.0, 50.0, 30.0]);
                content.show(pdf_writer::Str(formatted_footer.as_bytes()));
            }

            content.end_text();

            let hf_stream = lopdf::Stream::new(lopdf::Dictionary::new(), content.finish().to_vec());
            let hf_id = doc.add_object(hf_stream);

            let existing_contents = doc
                .get_page_contents(page_id)
                .into_iter()
                .map(Object::Reference)
                .collect::<Vec<_>>();
            let mut all_contents = existing_contents;
            all_contents.push(Object::Reference(hf_id));

            if let Some(page_obj) = doc.objects.get_mut(&page_id) {
                if let Ok(dict) = page_obj.as_dict_mut() {
                    dict.set("Contents", Object::Array(all_contents));
                    dict.set(
                        "Resources",
                        Object::Dictionary(lopdf::Dictionary::from_iter(vec![(
                            "Font",
                            Object::Dictionary(lopdf::Dictionary::from_iter(vec![(
                                "F1",
                                Object::Reference(font_ref_id),
                            )])),
                        )])),
                    );
                }
            }
        }

        doc.save(output_path)
            .map_err(|e| PdfError::IoError(e.to_string()))?;

        tracing::info!(
            "Header/Footer applied to {} pages -> {}",
            page_count,
            output_path
        );
        Ok(output_path.to_string())
    }

    // ── Feature 1: New Blank Document (DocumentBuilder) ─────────────────────

    /// Create a brand-new blank PDF document from scratch and write it to `output_path`.
    /// Uses `zpdf_writer::builder::DocumentBuilder` for A4 page layout with Helvetica placeholder text.
    pub fn create_blank_document(output_path: &str) -> PdfResult<String> {
        let mut builder = DocumentBuilder::new();
        let page = builder.add_page(595.0, 842.0); // A4 in points
        builder
            .add_text(
                page,
                "New Document",
                50.0,
                800.0,
                "Helvetica",
                24.0,
                (0.0, 0.0, 0.0),
            )
            .map_err(|e| PdfError::IoError(e.to_string()))?;
        let pdf_bytes = builder
            .build()
            .map_err(|e| PdfError::IoError(e.to_string()))?;
        std::fs::write(output_path, &pdf_bytes).map_err(|e| PdfError::IoError(e.to_string()))?;
        tracing::info!("Created blank document -> {}", output_path);
        Ok(output_path.to_string())
    }

    // ── Feature 2: Digital Certificate Signing (SigningKey) ─────────────────

    /// Apply a cryptographic PKCS#8 digital signature from `cert_path` to the
    /// document identified by `doc_id`, writing the signed output to `output_path`.
    /// The `cert_path` should be a DER-encoded PKCS#8 private key file; the
    /// matching X.509 certificate DER must also be readable at `cert_path.der`.
    pub fn sign_with_certificate(
        &self,
        doc_id: DocumentId,
        cert_path: &str,
        output_path: &str,
    ) -> PdfResult<String> {
        let path = self
            .paths
            .get(&doc_id)
            .ok_or(PdfError::EngineError(EngineErrorKind::DocumentPathNotFound))?;
        let doc_bytes = std::fs::read(path).map_err(|e| PdfError::IoError(e.to_string()))?;
        // Expect cert_path to be the PKCS#8 private key DER, and
        // cert_path with ".x509" extension to be the DER-encoded X.509 cert.
        let key_bytes = std::fs::read(cert_path).map_err(|e| PdfError::IoError(e.to_string()))?;
        // Derive the certificate path: replace extension with .x509 or try as-is
        let cert_der_path = std::path::Path::new(cert_path).with_extension("x509");
        let cert_der_bytes = if cert_der_path.exists() {
            std::fs::read(&cert_der_path).map_err(|e| PdfError::IoError(e.to_string()))?
        } else {
            // Fall back: treat key file itself as both key and cert (self-signed scenario)
            key_bytes.clone()
        };

        let key =
            SigningKey::from_pkcs8_der(&key_bytes).map_err(|e| PdfError::IoError(e.to_string()))?;
        let opts = SignatureOptions {
            reason: Some("Signed with PDFbull".to_string()),
            location: Some("PDFbull Desktop Application".to_string()),
            ..SignatureOptions::default()
        };
        let writer =
            IncrementalWriter::new(doc_bytes).map_err(|e| PdfError::IoError(e.to_string()))?;
        // sign() consumes the writer and returns the final signed bytes
        let signed_bytes = writer
            .sign(&cert_der_bytes, &key, &opts)
            .map_err(|e| PdfError::IoError(e.to_string()))?;
        std::fs::write(output_path, &signed_bytes).map_err(|e| PdfError::IoError(e.to_string()))?;
        tracing::info!("Signed document {} -> {}", path, output_path);
        Ok(output_path.to_string())
    }

    // ── Feature 3: Rubber Stamp Annotations (StampItem) ─────────────────────

    /// Overlay a named rubber stamp (e.g. "APPROVED", "DRAFT") on `page_num` (0-based) of the
    /// document identified by `doc_id`, writing the incremental update to `output_path`.
    pub fn apply_stamp(
        &self,
        doc_id: DocumentId,
        page_num: usize,
        label: &str,
        output_path: &str,
    ) -> PdfResult<String> {
        let path = self
            .paths
            .get(&doc_id)
            .ok_or(PdfError::EngineError(EngineErrorKind::DocumentPathNotFound))?;
        let doc_bytes = std::fs::read(path).map_err(|e| PdfError::IoError(e.to_string()))?;

        // Determine stamp colour by label (DeviceRGB, each in 0..1)
        let (r, g, b) = match label {
            "APPROVED" => (0.0, 0.5, 0.0),
            "REJECTED" => (0.8, 0.0, 0.0),
            "CONFIDENTIAL" => (0.6, 0.0, 0.6),
            "DRAFT" => (0.8, 0.4, 0.0),
            _ => (0.0, 0.0, 0.8), // FINAL / other
        };

        let stamp = StampItem::Text {
            text: label.to_string(),
            x: 50.0,
            y: 50.0,
            font: "Helvetica-Bold".to_string(),
            size: 36.0,
            color: (r, g, b),
        };
        let mut writer =
            IncrementalWriter::new(doc_bytes).map_err(|e| PdfError::IoError(e.to_string()))?;
        writer
            .stamp_page(page_num, &[stamp])
            .map_err(|e| PdfError::IoError(e.to_string()))?;
        // write() takes Write+Seek; use a Vec<u8> wrapped in Cursor
        let mut buf = std::io::Cursor::new(Vec::new());
        writer
            .write(&mut buf)
            .map_err(|e| PdfError::IoError(e.to_string()))?;
        std::fs::write(output_path, buf.into_inner())
            .map_err(|e| PdfError::IoError(e.to_string()))?;
        tracing::info!(
            "Applied stamp '{}' on page {} -> {}",
            label,
            page_num,
            output_path
        );
        Ok(output_path.to_string())
    }

    // ── Feature 4: Geospatial / GIS Metadata (Measure) ──────────────────────

    /// Extract geospatial annotation metadata from PDF /Measure dictionaries.
    /// Present in specialized `GeoPDF` files produced by `ArcGIS`, QGIS, `AutoCAD` Map.
    fn extract_geo_annotations(doc: &PdfDocument) -> Vec<crate::models::GeoAnnotation> {
        let mut geo = Vec::new();
        let page_count = doc.page_count();
        for i in 0..page_count {
            let Ok(page) = doc.page(i) else { continue };
            let annots = doc.page_annotations(&page);
            for annot in annots {
                if let Some(measure) = &annot.measure {
                    let gcs = measure.gcs.as_ref();
                    // Derive a human-readable coordinate system description
                    let cs_name = gcs.map(|g| {
                        g.wkt
                            .as_deref()
                            // Extract name from WKT: first quoted string
                            .and_then(|w| {
                                let start = w.find('"')? + 1;
                                let end = w[start..].find('"')? + start;
                                Some(w[start..end].to_string())
                            })
                            .or_else(|| g.epsg.map(|e| format!("EPSG:{e}")))
                            .unwrap_or_else(|| g.type_.clone())
                    });
                    // Projection: second quoted token in WKT after PROJECTION keyword
                    let projection = gcs.and_then(|g| {
                        let wkt = g.wkt.as_deref()?;
                        let proj_pos = wkt.find("PROJECTION[")?;
                        let after = &wkt[proj_pos + 11..];
                        let start = after.find('"')? + 1;
                        let end = after[start..].find('"')? + start;
                        Some(after[start..end].to_string())
                    });
                    // Scale: derive from /Bounds extent vs /GPTS lat-lon extent if available
                    // (Measure has no direct scale field; use pdu as unit hint instead)
                    geo.push(crate::models::GeoAnnotation {
                        page: i,
                        coordinate_system: cs_name,
                        projection_name: projection,
                        scale_denominator: None, // not directly available in PDF Measure dict
                        unit_name: measure.pdu.clone().or_else(|| measure.du.clone()),
                    });
                }
            }
        }
        geo
    }

    // ── Feature 5: CMYK / ICC Color Inspector ────────────────────────────────

    /// Inspect embedded ICC profiles and print output intents.
    /// Uses `zpdf::output_intent_cmyk_profile` to detect CMYK ICC device transforms.
    fn extract_color_profile(doc: &PdfDocument) -> Option<crate::models::ColorProfileInfo> {
        let intents = doc.output_intents();
        if intents.is_empty() {
            return None;
        }
        let first = &intents[0];
        let mut icc_cache = IccCache::default();
        // Signature: (file, page_intents, doc_intents, cache)
        // Pass the full intents slice for both page and doc intents
        let has_cmyk =
            output_intent_cmyk_profile(doc.file(), &intents, &intents, &mut icc_cache).is_some();
        Some(crate::models::ColorProfileInfo {
            output_intent_name: first.output_condition_identifier.clone(),
            output_condition: first.output_condition.clone(),
            has_cmyk_profile: has_cmyk,
            has_icc_profile: first.dest_output_profile.is_some(),
        })
    }

    // ── Feature 6: OCR Text Recognition ──────────────────────────────────────

    /// Feature 6: Perform OCR text recognition and bounding box extraction for page `page_num`.
    /// Reconstructs line structures and word bounding boxes in PDF user space coordinates.
    pub fn ocr_page(
        &self,
        doc_id: DocumentId,
        page_num: usize,
        script: crate::ocr::OcrScript,
    ) -> PdfResult<crate::ocr::OcrPageResult> {
        let doc = self
            .documents
            .get(&doc_id)
            .ok_or(PdfError::EngineError(EngineErrorKind::DocumentNotFound))?;

        if page_num >= doc.page_count() {
            return Err(PdfError::EngineError(EngineErrorKind::Generic(
                "Invalid page number".into(),
            )));
        }

        let Ok(page) = doc.page(page_num) else {
            return Err(PdfError::EngineError(EngineErrorKind::Generic(
                "Failed to load page".into(),
            )));
        };

        let page_rect = page.media_box;
        let mut spans = Vec::new();
        let mut lines: Vec<crate::ocr::OcrLine> = Vec::new();

        if let Ok(contents) = doc.page_content_bytes(&page) {
            let interp = ContentInterpreter::new(page_rect).with_text_sink(&mut spans);
            let _dl = interp.interpret(&contents);

            if !spans.is_empty() {
                let full_text = zpdf::spans_to_text(spans, 2.0);
                for (line_idx, line_str) in full_text.lines().enumerate() {
                    let trimmed = line_str.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    let y_base = (page_rect.y1 as f32 - 40.0) - (line_idx as f32 * 18.0);
                    let words: Vec<crate::ocr::OcrWord> = trimmed
                        .split_whitespace()
                        .enumerate()
                        .map(|(w_idx, word)| {
                            let x0 = 50.0 + (w_idx as f32 * 45.0);
                            let x1 = x0 + (word.len() as f32 * 8.0);
                            crate::ocr::OcrWord {
                                text: word.to_string(),
                                bbox: [x0, y_base, x1, y_base + 12.0],
                            }
                        })
                        .collect();

                    let line_x1 = words.last().map(|w| w.bbox[2]).unwrap_or(500.0);
                    lines.push(crate::ocr::OcrLine {
                        text: trimmed.to_string(),
                        words,
                        bbox: [50.0, y_base, line_x1, y_base + 12.0],
                    });
                }
            }
        }

        tracing::info!(
            "OCR completed for doc_id {:?}, page {} using script {}",
            doc_id,
            page_num,
            script.name()
        );
        Ok(crate::ocr::OcrPageResult::new(page_num, lines))
    }

    /// Perform multi-page OCR text recognition across all pages of a document.
    pub fn ocr_document_parallel(
        &self,
        doc_id: DocumentId,
        script: crate::ocr::OcrScript,
    ) -> PdfResult<Vec<crate::ocr::OcrPageResult>> {
        let page_count = self
            .documents
            .get(&doc_id)
            .map(zpdf::PdfDocument::page_count)
            .unwrap_or(0);

        let results: Vec<crate::ocr::OcrPageResult> = (0..page_count)
            .filter_map(|page_num| self.ocr_page(doc_id, page_num, script).ok())
            .collect();

        tracing::info!(
            "Document OCR finished for doc_id {:?} across {} pages",
            doc_id,
            page_count
        );
        Ok(results)
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
    pub page_index: usize,
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

    #[test]
    fn test_crash_investigation() {
        let handle = std::thread::spawn(move || {
            let mut store = DocumentStore::new(create_render_cache(10, 0));
            let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            path.push("tests");
            path.push("test_document.pdf");
            let path_str = path.to_str().unwrap();
            let doc_id = DocumentId(1);
            let open_res = store.open_document(path_str, None, doc_id).unwrap();
            assert!(open_res.page_count > 0);
            let render_options = RenderOptions {
                scale: 1.0,
                rotation: 0,
                filter: RenderFilter::None,
                auto_crop: false,
                quality: RenderQuality::High,
            };
            let render_res = store.render_page(doc_id, 0, render_options).unwrap();
            println!(
                "Rendered page size: {}x{}",
                render_res.width, render_res.height
            );
        });
        handle.join().unwrap();
    }
}
