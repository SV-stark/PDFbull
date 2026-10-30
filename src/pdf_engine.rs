use crate::models::{
    Annotation, AnnotationStyle, DocumentId, EngineErrorKind, FormField, FormFieldVariant,
    Hyperlink, PdfError, PdfResult, SearchResultItem,
};
use lopdf::{Document, Object, ObjectId};
use ocrs::TextItem;
use quick_cache::{Weighter, sync::Cache};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::Arc;
use zpdf::{
    AnnotationSpec, ContentInterpreter, FieldKind, FieldValue, FormFiller, IccCache, ImageCache,
    IncrementalWriter, MarkupKind, ParseLimits, PdfDocument, Rect as ZpdfRect, RenderBackend,
    cpu::CpuRenderer, detect_tables, detect_tables_with_rules, output_intent_cmyk_profile,
    search_spans, spans_to_text, struct_ordered_text,
};
use zpdf::{RewriteOptions, rewrite_pdf};

/// Distil zpdf's [`DestView`] into the view the app can actually act on.
///
/// zpdf exposes eight variants; a reader at this zoom range can honour two
/// shapes — "jump to a point" (`XYZ`) and "show the whole page" (every `Fit*`
/// and bounding-box variant). Collapsing the rest is deliberate: pretending to
/// support `FitR`'s exact quad would require a zoom-to-fit-rect path the
/// canvas does not have, and a `ScrollTo` with the rect's top-left is a better
/// lie than nothing.
impl crate::models::DestinationView {
    pub fn from_dest_view(view: &zpdf::DestView, has_page: bool) -> Self {
        if !has_page {
            // A `GoToR`/remote action has no page in this document; there is
            // nothing to scroll to.
            return Self::WholePage;
        }
        match view {
            zpdf::DestView::Xyz { left, top, zoom } => Self::ScrollTo {
                x: *left,
                y: *top,
                zoom: *zoom,
            },
            _ => Self::WholePage,
        }
    }

    /// Short label for the sidebar row.
    pub fn label(&self) -> &'static str {
        match self {
            Self::ScrollTo { .. } => "XYZ",
            Self::WholePage => "Fit",
        }
    }
}
use zpdf_writer::{
    builder::DocumentBuilder,
    redact::{RedactOptions, redact_pdf},
    sign::{SignatureOptions, SigningKey},
    stamp::StampItem,
};
use zune_image::codecs::ImageFormat;
use zune_image::image::Image;

/// Converts a hex color string like "#FF5500" into normalized RGB floats (0.0..=1.0).
pub fn hex_to_rgb(hex: &str) -> (f32, f32, f32) {
    let hex = hex.trim_start_matches('#');
    // `len()` is a *byte* count, but the slices below assume ASCII. A 6-byte
    // string such as "€€" passes a bare `len() == 6` test and then panics on
    // `&hex[0..2]` ("byte index 2 is not a char boundary"). Parse from bytes
    // with an explicit ASCII-hex-digit guard instead, which cannot panic and
    // simply treats any non-hex input as black.
    let bytes = hex.as_bytes();
    if bytes.len() != 6 || !bytes.iter().all(u8::is_ascii_hexdigit) {
        return (0.0, 0.0, 0.0);
    }
    let pair = |i: usize| -> f32 {
        let hi = (bytes[i] as char).to_digit(16).unwrap_or(0);
        let lo = (bytes[i + 1] as char).to_digit(16).unwrap_or(0);
        ((hi * 16 + lo) as f32) / 255.0
    };
    (pair(0), pair(2), pair(4))
}

/// Largest file any of the whole-file processing entry points will load.
///
/// `open_document` and `convert_pdf_doc` already enforced this; every other
/// path read the file with no bound at all, so a 4 GB pick in the Optimize (or
/// Encrypt, or Merge, or Split) dialog bypassed the guard entirely and the
/// process was OOM-killed. `optimize_pdf` was the worst case: it held the input,
/// a clone of the input, and the staged rewrite simultaneously.
pub const MAX_INPUT_BYTES: u64 = 512 * 1024 * 1024;

/// Read a whole input file, refusing anything over `MAX_INPUT_BYTES`.
///
/// The size is checked via metadata *before* the read, so an oversized file is
/// rejected without first allocating room for it.
pub fn read_input_capped(path: &str) -> PdfResult<Vec<u8>> {
    let meta = std::fs::metadata(path)
        .map_err(|e| PdfError::OpenFailed(format!("cannot stat {path}: {e}")))?;
    if meta.len() > MAX_INPUT_BYTES {
        return Err(PdfError::OpenFailed(format!(
            "{path} is {:.1} MB, above the {} MB limit for this operation",
            meta.len() as f64 / (1024.0 * 1024.0),
            MAX_INPUT_BYTES / (1024 * 1024)
        )));
    }
    std::fs::read(path).map_err(|e| PdfError::OpenFailed(e.to_string()))
}

/// The box a page's content is actually drawn into, as `(x0, y0, x1, y1)`.
///
/// `/MediaBox` and `/CropBox` are inheritable page attributes: either may be
/// declared once on the `/Pages` node and be absent from every individual page
/// dict. Reading the page dict alone therefore fell back to A4 for exactly the
/// documents that declare their size the standard way.
///
/// `/CropBox` takes precedence, because that — not `/MediaBox` — is what the
/// viewer shows and therefore what generated content must be positioned against.
fn effective_page_box(doc: &Document, page_id: ObjectId) -> Option<(f32, f32, f32, f32)> {
    // Walk /Parent for an inherited key. Bounded so a /Parent cycle (which
    // malformed files do contain) cannot spin forever.
    let inherited = |key: &[u8]| -> Option<Object> {
        let mut current = page_id;
        for _ in 0..64 {
            let dict = doc.objects.get(&current).and_then(|o| o.as_dict().ok())?;
            if let Ok(v) = dict.get(key) {
                return Some(v.clone());
            }
            current = match dict.get(b"Parent").ok()? {
                Object::Reference(r) => *r,
                _ => return None,
            };
        }
        None
    };

    let as_box = |obj: &Option<Object>| -> Option<(f32, f32, f32, f32)> {
        let arr = match obj {
            Some(Object::Array(a)) if a.len() == 4 => a,
            Some(Object::Reference(r)) => match doc.objects.get(r) {
                Some(Object::Array(a)) if a.len() == 4 => a,
                _ => return None,
            },
            _ => return None,
        };
        let num = |o: &Object| -> Option<f32> {
            match o {
                Object::Real(v) => Some(*v),
                Object::Integer(v) => Some(*v as f32),
                _ => None,
            }
        };
        let (x0, y0) = (num(&arr[0])?, num(&arr[1])?);
        let (x1, y1) = (num(&arr[2])?, num(&arr[3])?);
        // Normalize so a box given as [top-right, bottom-left] still yields a
        // positive extent.
        Some((x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)))
    };

    as_box(&inherited(b"CropBox")).or_else(|| as_box(&inherited(b"MediaBox")))
}

/// The effective rotation of a page in a lopdf Document, taking inheritance into account.
fn page_rotation_lopdf(doc: &Document, page_id: ObjectId) -> i32 {
    let mut curr = Some(page_id);
    while let Some(id) = curr {
        if let Ok(dict) = doc.get_dictionary(id) {
            if let Ok(rot) = dict.get(b"Rotate").and_then(Object::as_i64) {
                return ((rot % 360 + 360) % 360) as i32;
            }
            curr = dict.get(b"Parent").and_then(Object::as_reference).ok();
        } else {
            break;
        }
    }
    0
}

/// Write generated PDF bytes to `path` without ever truncating the destination
/// before the new content exists.
///
/// Every PDF-producing path goes through here rather than `fs::write`. The
/// reason is concrete: several of these commands default to overwriting their
/// own input (the save dialog can hand back the path that was opened), and
/// `fs::write` truncates before it writes — so a failure part-way through left
/// the user with a zero-length file and no copy of the original.
fn write_pdf_output(path: &str, bytes: &[u8]) -> PdfResult<()> {
    crate::storage::atomic_write_bytes(std::path::Path::new(path), bytes)
        .map_err(|e| PdfError::IoError(e.to_string()))
}

/// Encode `text` for a PDF literal string shown with a standard Type1 font.
///
/// `pdf_writer::Str` writes its bytes verbatim as a PDFDocEncoding literal
/// string. Handing it `str::as_bytes()` therefore put raw UTF-8 into a field
/// readers interpret as single-byte codes, so `机密` became three unrelated
/// glyphs per character.
///
/// WinAnsiEncoding is the encoding every standard-14 font defaults to, and it
/// covers Latin-1, so accented Latin text maps almost one-to-one. Anything
/// outside it cannot be drawn with a non-embedded Helvetica; it is replaced with
/// `?` and reported, rather than silently rendering as mojibake.
fn encode_pdf_text(text: &str) -> (Vec<u8>, usize) {
    let mut out = Vec::with_capacity(text.len());
    let mut unmappable = 0usize;
    for ch in text.chars() {
        match ch {
            // Printable ASCII passes through unchanged.
            c if (0x20..0x7F).contains(&(c as u32)) => out.push(c as u8),
            // WinAnsiEncoding agrees with Latin-1 across 0xA0..=0xFF, except
            // for the handful of code points Windows remapped.
            '€' => out.push(0x80),
            '‚' => out.push(0x82),
            'ƒ' => out.push(0x83),
            '„' => out.push(0x84),
            '…' => out.push(0x85),
            '†' => out.push(0x86),
            '‡' => out.push(0x87),
            'ˆ' => out.push(0x88),
            '‰' => out.push(0x89),
            'Š' => out.push(0x8A),
            '‹' => out.push(0x8B),
            'Œ' => out.push(0x8C),
            c if (0xA0..=0xFF).contains(&(c as u32)) => out.push(c as u8),
            // A literal string needs these escaped or they break the syntax.
            _ => {
                unmappable += 1;
                out.push(b'?');
            }
        }
    }
    (out, unmappable)
}

/// Map every byte offset in `text.to_lowercase()` back to the corresponding byte
/// offset in `original`.
///
/// `str::to_lowercase` is not length-preserving. It applies full Unicode
/// special-casing, so some characters expand: `İ` (U+0130) becomes `i̇`
/// (U+0069 U+0307), one char into two, and the Greek final-sigma rule folds a
/// trailing `Σ` into `ς`. Any code that indexes the original string using an
/// offset computed against the lowercase string is wrong after such a
/// character — which is why a search for `x` in `"İx"` silently returned no
/// hit.
///
/// Both strings are produced by the *same* `to_lowercase` call per input char,
/// so walking the original char by char and lowercasing each one reproduces the
/// lowercase string exactly and yields an exact offset mapping.
///
/// The returned map always contains an entry for `original.len()` (the end
/// sentinel). Where a lowercase char boundary does not correspond to an original
/// char boundary, the mapping records the *start* of the original character that
/// the boundary falls inside, so a range lookup can never produce a reversed or
/// out-of-bounds slice.
///
/// Reduce an arbitrary angle to one of 0/90/180/270, the only values a PDF
/// `/Rotate` can meaningfully take.
///
/// A negative or out-of-range `/Rotate` (or a sum with the user's view rotation)
/// still has to select one of the four axis-aligned quadrants, otherwise the
/// overlay transform would fall through to the identity and misplace everything.
pub fn normalize_rotation(degrees: i32) -> i32 {
    degrees.rem_euclid(360) / 90 * 90
}

/// Map a rect measured on a *rendered* page bitmap back into the page's
/// user-space coordinates, `[x0, y0, x1, y1]` with `y0 < y1` (y up).
///
/// `render_page_internal` rasterizes the page's **effective** (crop) box with
/// the page's own `/Rotate` baked in, so the bitmap is not `box_w × box_h`:
/// for a quarter turn the axes swap. `ocr_page` used to scale OCR pixels by the
/// unrotated `media_box` on both axes, which misplaced every word on a rotated
/// page and used the wrong box entirely on a cropped one.
///
/// `rect` is the OCR pixel rect (top-left origin), `bitmap` is the rendered
/// bitmap's `(width, height)`, `box_rect` is the effective box as
/// `(x0, y0, x1, y1)`, and `rotation` is the page's `/Rotate`.
///
/// The rotation arms are the exact inverse of
/// [`crate::ui_gpui::canvas::overlay_rect_in_rotated_space`]; changing one
/// without the other desynchronises OCR from the text layer and every overlay.
pub fn rotated_bitmap_rect_to_page_space(
    rect: (f32, f32, f32, f32),
    bitmap: (u32, u32),
    box_rect: (f32, f32, f32, f32),
    rotation: i32,
) -> (f32, f32, f32, f32) {
    let (box_x0, box_y0, box_x1, box_y1) = box_rect;
    let box_w = (box_x1 - box_x0).abs();
    let box_h = (box_y1 - box_y0).abs();
    let rot = normalize_rotation(rotation);

    // The bitmap's axes are swapped for a quarter turn; its points-per-pixel
    // therefore follow the *display* size, not the unrotated box size.
    let (display_w, display_h) = if rot == 90 || rot == 270 {
        (box_h, box_w)
    } else {
        (box_w, box_h)
    };
    let sx = if bitmap.0 == 0 {
        0.0
    } else {
        display_w / bitmap.0 as f32
    };
    let sy = if bitmap.1 == 0 {
        0.0
    } else {
        display_h / bitmap.1 as f32
    };

    // Pixels -> display (rotated) space, in points.
    let dx = rect.0 * sx;
    let dy = rect.1 * sy;
    let dw = (rect.2 - rect.0) * sx;
    let dh = (rect.3 - rect.1) * sy;

    // Display space -> unrotated box space, top-left origin. Inverse of
    // `overlay_rect_in_rotated_space`.
    let (ux, uy, uw, uh) = match rot {
        90 => (dy, box_h - dx - dw, dh, dw),
        180 => (box_w - dx - dw, box_h - dy - dh, dw, dh),
        270 => (box_w - dy - dh, dx, dh, dw),
        _ => (dx, dy, dw, dh),
    };

    // Top-left box space -> PDF user space (y up).
    let x0 = box_x0 + ux;
    let x1 = box_x0 + ux + uw;
    let y1 = box_y1 - uy;
    let y0 = box_y1 - uy - uh;
    (x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1))
}

/// Build a `zpdf::OcConfig` with the user's visibility overrides applied **on
/// top of** the document's own optional-content configuration.
///
/// The overrides must be layered over `base_oc`, not substituted for it. The
/// previous version rebuilt `/OCProperties /D` from `overrides` alone, with a
/// hard-coded `/BaseState /ON` — so `base_oc` (the config actually decoded from
/// the document at open time) was consulted only as an error fallback. A
/// document whose `/D` hid three layers by default opened correctly, and the
/// first toggle of any *other* layer produced a config that no longer listed
/// those three in `/OFF`; with `BaseState ON` they all became visible. One
/// toggle silently revealed every layer the author had hidden.
///
/// `zpdf::OcConfig` keeps its `/ON`, `/OFF` and `/BaseState` private, so the
/// base state cannot be read back field-by-field. Instead the *resolved*
/// visibility of every known group is probed through the one public accessor,
/// `group_visible`, and re-expressed as explicit `/ON` + `/OFF` entries — which
/// is what the synthesized config means anyway. `known_groups` must be the
/// document's full optional-content group set; a group missing from it cannot
/// be carried over.
///
/// A config is synthesized through a real `lopdf` catalog rather than
/// transmuting memory, and parsed back by `zpdf`, so no `unsafe` is involved.
pub fn apply_oc_overrides(
    base_oc: &zpdf::OcConfig,
    overrides: &std::collections::HashMap<zpdf::ObjectId, bool>,
    known_groups: &[zpdf::ObjectId],
) -> zpdf::OcConfig {
    // With nothing to override, hand back the document's own config untouched.
    // This is also the only path that can preserve the base `/BaseState` value
    // exactly, so it must stay a true pass-through.
    if overrides.is_empty() {
        return base_oc.clone();
    }

    let mut doc = lopdf::Document::with_version("1.4");
    let pages_id = doc.new_object_id();
    let catalog_id = doc.new_object_id();

    let mut d_dict = lopdf::Dictionary::new();
    // BaseState ON is correct for a config that enumerates every group
    // explicitly: nothing is left to the default.
    d_dict.set("BaseState", lopdf::Object::Name(b"ON".to_vec()));

    // Carry the document's own resolved visibility forward, then apply the
    // user's toggles on top. Both lists are sorted, so the emitted arrays are
    // deterministic -- `overrides` is a `HashMap`, and iterating it raw made
    // the /ON//OFF order vary between runs of the same document.
    let mut on_ids: Vec<zpdf::ObjectId> = Vec::new();
    let mut off_ids: Vec<zpdf::ObjectId> = Vec::new();

    for &id in known_groups {
        let visible = match overrides.get(&id) {
            Some(&user_choice) => user_choice,
            // No user override: keep whatever the document itself declared.
            None => base_oc.group_visible(id),
        };
        if visible {
            on_ids.push(id);
        } else {
            off_ids.push(id);
        }
    }

    on_ids.sort();
    off_ids.sort();

    d_dict.set(
        "ON",
        lopdf::Object::Array(
            on_ids
                .into_iter()
                .map(|id| lopdf::Object::Reference((id.0, id.1)))
                .collect(),
        ),
    );
    d_dict.set(
        "OFF",
        lopdf::Object::Array(
            off_ids
                .into_iter()
                .map(|id| lopdf::Object::Reference((id.0, id.1)))
                .collect(),
        ),
    );

    let mut ocp_dict = lopdf::Dictionary::new();
    ocp_dict.set("D", lopdf::Object::Dictionary(d_dict));

    let mut catalog = lopdf::Dictionary::new();
    catalog.set("Type", lopdf::Object::Name(b"Catalog".to_vec()));
    catalog.set("Pages", lopdf::Object::Reference(pages_id));
    catalog.set("OCProperties", lopdf::Object::Dictionary(ocp_dict));

    doc.objects
        .insert(catalog_id, lopdf::Object::Dictionary(catalog));
    doc.trailer
        .set("Root", lopdf::Object::Reference(catalog_id));

    let page_id = doc.new_object_id();
    let mut page = lopdf::Dictionary::new();
    page.set("Type", lopdf::Object::Name(b"Page".to_vec()));
    page.set("Parent", lopdf::Object::Reference(pages_id));
    page.set(
        "MediaBox",
        lopdf::Object::Array(vec![
            lopdf::Object::Integer(0),
            lopdf::Object::Integer(0),
            lopdf::Object::Integer(100),
            lopdf::Object::Integer(100),
        ]),
    );
    doc.objects.insert(page_id, lopdf::Object::Dictionary(page));

    let mut pages = lopdf::Dictionary::new();
    pages.set("Type", lopdf::Object::Name(b"Pages".to_vec()));
    pages.set(
        "Kids",
        lopdf::Object::Array(vec![lopdf::Object::Reference(page_id)]),
    );
    pages.set("Count", lopdf::Object::Integer(1));
    doc.objects
        .insert(pages_id, lopdf::Object::Dictionary(pages));

    let mut bytes = Vec::new();
    if doc.save_to(&mut bytes).is_err() {
        return base_oc.clone();
    }

    if let Ok(pdf_doc) = zpdf::PdfDocument::open(bytes)
        && let Some(oc) = pdf_doc.oc_config()
    {
        oc
    } else {
        base_oc.clone()
    }
}

// PDF field-flags bit for "radio button" (ISO 32000-1 Table 221).
const FF_RADIO: i64 = 1 << 15;
const WHITE_THRESHOLD: u8 = 245;
const BBOX_MARGIN: u32 = 10;
const NO_SHADOW_THRESHOLD: u8 = 230;

/// `/NM` prefix stamped on every annotation this app writes. A re-save strips
/// all annotations carrying this marker before appending the current set, so
/// `Ctrl+S` is idempotent instead of appending a fresh copy on every press.
const PDFBULL_ANNOT_MARKER: &str = "PDFBULL:";

/// Returns true when `obj` is an annotation dictionary previously written by
/// this application (identified by its `/NM` marker).
fn is_pdfbull_annotation(obj: &zpdf::PdfObject) -> bool {
    let Ok(dict) = obj.as_dict() else {
        return false;
    };
    match dict.get("NM") {
        Some(zpdf::PdfObject::String(s)) => {
            String::from_utf8_lossy(&s.0).starts_with(PDFBULL_ANNOT_MARKER)
        }
        Some(zpdf::PdfObject::Name(n)) => n.as_str().starts_with(PDFBULL_ANNOT_MARKER),
        _ => false,
    }
}

/// [`is_pdfbull_annotation`] for a `lopdf` dictionary, used on the read path.
///
/// `load_annotations` parses with `lopdf` while `save_annotations` strips with
/// `zpdf`, so the same marker has to be recognised in both object models.
fn is_pdfbull_annotation_lopdf(dict: &lopdf::Dictionary) -> bool {
    dict.get(b"NM")
        .ok()
        .and_then(|obj| obj.as_str().ok())
        .is_some_and(|nm| nm.starts_with(PDFBULL_ANNOT_MARKER.as_bytes()))
}

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct RenderKey {
    pub doc_id: DocumentId,
    pub page_num: usize,
    pub scale: u32,
    pub rotation: i32,
    pub auto_crop: bool,
    pub quality: RenderQuality,
    /// Colour transform applied after rasterising.
    ///
    /// This belongs in the key. Without it, toggling Midnight mode — which
    /// switches the filter to `Inverted` — produced a key identical to the
    /// already-cached unfiltered render, so the engine returned the cached
    /// bitmap from memory and the pages stayed exactly as they were. The view
    /// invalidated its own bitmaps, but the engine cache answered the
    /// re-request.
    pub filter: RenderFilter,
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
    doc_keys: std::sync::Mutex<HashMap<DocumentId, std::collections::HashSet<RenderKey>>>,
    /// Monotonic invalidation counter per document.
    ///
    /// Renders capture the value before they start and re-check it before
    /// caching, so a render that was already in flight when the document changed
    /// cannot re-insert a stale bitmap after the invalidation. Needed because
    /// all workers share one cache while holding independent document state.
    generations: std::sync::Mutex<HashMap<DocumentId, u64>>,
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
            doc_keys: std::sync::Mutex::new(HashMap::new()),
            generations: std::sync::Mutex::new(HashMap::new()),
        }
    }

    pub fn get(&self, key: &RenderKey) -> Option<crate::models::RenderResult> {
        self.cache.get(key)
    }

    /// Record `key` against its document, then cache `result`.
    ///
    /// The key set is **bounded** per document. `doc_keys` used to retain every
    /// key ever rendered for the life of the document, which is the exact
    /// unbounded-growth problem its own comment said had been fixed by dropping
    /// an earlier per-document `Vec<RenderKey>`. Continuous wheel zooming mints
    /// a distinct `RenderKey` per quantised scale crossed with rotation, filter
    /// and quality -- tens of thousands on a long read -- and `quick_cache`
    /// evicts by weight without ever notifying `doc_keys`, so the accounting
    /// defeat the comment described was still in force.
    ///
    /// When the cap is hit the oldest tracked key is dropped from the set. That
    /// only costs the ability to eagerly invalidate that one entry on a
    /// document mutation; it is not a correctness problem, because the cache's
    /// own eviction handles the bitmap and a stale entry is bounded in count
    /// rather than in age.
    fn track_key(&self, key: &RenderKey) {
        let mut guard = match self.doc_keys.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        let set = guard.entry(key.doc_id).or_default();
        set.insert(key.clone());
        while set.len() > MAX_TRACKED_KEYS_PER_DOC {
            // `HashSet` has no order, so pick any survivor to evict. Cheap, and
            // avoids making this a `Vec`/`BTreeSet` with its own bookkeeping.
            if let Some(victim) = set.iter().next().cloned() {
                set.remove(&victim);
            } else {
                break;
            }
        }
    }

    pub fn put(&self, key: RenderKey, result: crate::models::RenderResult) {
        self.track_key(&key);
        self.cache.insert(key, result);
    }

    pub fn remove(&self, key: &RenderKey) {
        let mut guard = match self.doc_keys.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        if let Some(set) = guard.get_mut(&key.doc_id) {
            set.remove(key);
        }
        self.cache.remove(key);
    }

    /// Drop every cached bitmap for `doc_id`, whether or not its keys are still
    /// tracked.
    ///
    /// `remove_document`/`invalidate_document` both depend on the tracked-key
    /// set being intact. Every lock acquisition here used to be
    /// `if let Ok(..)`, so a single poisoned mutex turned both into silent
    /// no-ops: `close_document` stopped releasing a closed tab's bitmaps (they
    /// stayed resident in the shared budget for the process's lifetime) and
    /// `toggle_layer` stopped taking effect -- with no error anywhere. The
    /// poisoning is now recovered from explicitly and logged, so a poisoned
    /// lock degrades to "no eager invalidation" rather than "silently wrong".
    fn drop_document_keys(&self, doc_id: DocumentId) {
        let keys = match self.doc_keys.lock() {
            Ok(mut guard) => guard.remove(&doc_id),
            Err(poisoned) => {
                tracing::warn!(
                    "render-cache key index was poisoned; recovering it. Cached bitmaps \
                     for closed documents may not be released eagerly until it is rebuilt."
                );
                let mut guard = poisoned.into_inner();
                guard.remove(&doc_id)
            }
        };
        if let Some(keys) = keys {
            for key in keys {
                self.cache.remove(&key);
            }
        }
    }

    pub fn invalidate_document(&self, doc_id: DocumentId) {
        // Bump the generation *first*. A render already in flight on another
        // worker captured the old value before it started and will compare
        // against it after finishing, so it can tell that its result predates
        // this invalidation and decline to cache it.
        //
        // Without this, `ToggleLayer` is broadcast to every worker, each of which
        // invalidates; a worker that had not yet dequeued the broadcast still
        // held the pre-toggle layer state, rendered, and re-inserted a stale
        // bitmap *after* the invalidation ran. `invalidate_document` can only
        // remove keys present at the instant it runs, so the stale entry survived
        // and toggling a layer off appeared to do nothing.
        self.bump_generation(doc_id);
        self.drop_document_keys(doc_id);
    }

    pub fn remove_document(&self, doc_id: DocumentId) {
        self.bump_generation(doc_id);
        self.drop_document_keys(doc_id);
    }

    /// Current invalidation generation for `doc_id`.
    ///
    /// A render captures this before it starts and passes it back to `put`; a
    /// mismatch means the document changed mid-render and the result is stale.
    pub fn generation(&self, doc_id: DocumentId) -> u64 {
        match self.generations.lock() {
            Ok(guard) => guard.get(&doc_id).copied().unwrap_or(0),
            Err(poisoned) => poisoned.into_inner().get(&doc_id).copied().unwrap_or(0),
        }
    }

    fn bump_generation(&self, doc_id: DocumentId) {
        match self.generations.lock() {
            Ok(mut guard) => {
                *guard.entry(doc_id).or_insert(0) += 1;
            }
            Err(poisoned) => {
                *poisoned.into_inner().entry(doc_id).or_insert(0) += 1;
            }
        }
    }

    /// Cache `result` only if the document has not been invalidated since
    /// `expected_generation` was read.
    ///
    /// Returns `true` when the entry was stored.
    pub fn put_if_current(
        &self,
        key: RenderKey,
        result: crate::models::RenderResult,
        expected_generation: u64,
    ) -> bool {
        if self.generation(key.doc_id) != expected_generation {
            tracing::debug!(
                "discarding a render for {:?} that finished after the document changed",
                key.doc_id
            );
            return false;
        }
        self.track_key(&key);
        self.cache.insert(key, result);
        true
    }

    /// Number of render keys currently tracked for `doc_id`, or `None` when the
    /// document is not tracked at all.
    ///
    /// Exposed so a test can observe that invalidation releases this
    /// bookkeeping, not just the cached bitmaps.
    pub fn tracked_key_count(&self, doc_id: DocumentId) -> Option<usize> {
        match self.doc_keys.lock() {
            Ok(guard) => guard.get(&doc_id).map(std::collections::HashSet::len),
            Err(poisoned) => poisoned
                .into_inner()
                .get(&doc_id)
                .map(std::collections::HashSet::len),
        }
    }

    /// Total number of documents with a non-empty tracked-key set.
    ///
    /// Exposed for tests and diagnostics: it should never exceed the number of
    /// open documents, since `close_document` removes the entry.
    pub fn tracked_document_count(&self) -> usize {
        match self.doc_keys.lock() {
            Ok(guard) => guard.values().filter(|s| !s.is_empty()).count(),
            Err(poisoned) => poisoned
                .into_inner()
                .values()
                .filter(|s| !s.is_empty())
                .count(),
        }
    }
}

/// Upper bound on render keys tracked per document.
///
/// Sized well above what a normal browsing session mints for one document, while
/// still capping the bookkeeping at a few hundred KB per open tab.
const MAX_TRACKED_KEYS_PER_DOC: usize = 512;

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
    passwords: HashMap<DocumentId, String>,
    render_cache: SharedRenderCache,
    /// Base `OcConfig` as decoded from the document (read-only after load).
    oc_configs: HashMap<DocumentId, zpdf::OcConfig>,
    /// Per-document user visibility overrides applied on top of `oc_configs`.
    /// true = force ON, false = force OFF.
    oc_visibility: HashMap<DocumentId, HashMap<zpdf::ObjectId, bool>>,
    /// Every optional-content group id declared by each document, in the order
    /// `/OCProperties /OCGs` lists them.
    ///
    /// `zpdf::OcConfig` keeps its `/ON`, `/OFF` and `/BaseState` private, so the
    /// only way to rebuild a config that preserves the document's own layer
    /// states is to enumerate the groups and probe each one's resolved
    /// visibility. Without this list a toggle would silently reset every layer
    /// the user had not explicitly touched — see [`apply_oc_overrides`].
    oc_groups: HashMap<DocumentId, Vec<zpdf::ObjectId>>,
    /// Persistent per-document image cache to avoid re-decoding streams on every render.
    ///
    /// Bounded by [`DocumentStore::image_cache_bytes`] — see
    /// [`DEFAULT_IMAGE_CACHE_BYTES`] for why an unbounded cache is not usable
    /// here.
    image_caches: HashMap<DocumentId, ImageCache>,
    /// Aggregate decoded-image budget applied to every interpreter this store
    /// runs, in bytes.
    image_cache_bytes: u64,
    /// The on-disk revision each loaded document corresponds to. A worker
    /// reopens the file whenever the shared generation moves past this.
    loaded_generations: HashMap<DocumentId, u64>,
}

/// Decoded-image budget per open document.
///
/// `zpdf`'s `ParseLimits::default()` sets `max_image_cache_bytes` to **1 GiB**,
/// and `ContentInterpreter::new` always starts from those defaults unless
/// `with_image_cache_limit` is called. Because this store hands the interpreter
/// a *document-lifetime* `ImageCache` (so scrolling does not re-decode the same
/// stream on every frame), the 1 GiB default is a hard ceiling on how much the
/// process will retain per open tab.
///
/// That is wildly disproportionate to the input: a 2 MB scanned PDF holds a
/// few hundred KB of JBIG2/JPEG, but each 300 DPI page decodes to roughly
/// 2480x3508x4 = **35 MB** of RGBA. Scrolling through ~30 pages therefore
/// filled a gigabyte of decoded pixels that was never released until the tab
/// closed — and with the 512 MB page-render cache on top, a 2 MB file cost
/// ~1.5 GB of live allocations.
///
/// 96 MB holds a few pages of high-DPI scans, which is all the re-render path
/// actually benefits from; beyond that, re-decoding is far cheaper than the
/// resident memory.
const DEFAULT_IMAGE_CACHE_BYTES: u64 = 96 * 1024 * 1024;

// DocumentState wrapper removed as it was a single-field struct.

static OCR_ENGINE_CACHE: std::sync::LazyLock<
    std::sync::Mutex<HashMap<crate::ocr::OcrScript, Arc<ocrs::OcrEngine>>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));

fn get_or_load_ocr_engine(script: crate::ocr::OcrScript) -> PdfResult<Arc<ocrs::OcrEngine>> {
    if let Ok(guard) = OCR_ENGINE_CACHE.lock()
        && let Some(engine) = guard.get(&script)
    {
        return Ok(engine.clone());
    }

    if !script.is_model_available() {
        return Err(PdfError::EngineError(EngineErrorKind::Generic(format!(
            "OCR models for {} not found in {:?}",
            script.name(),
            crate::ocr::OcrScript::resolve_model_dir()
        ))));
    }

    let det_path = script.resolve_det_model_path();
    let rec_path = script.resolve_rec_model_path();
    let detection_model = rten::Model::load_file(&det_path).map_err(|e| {
        PdfError::EngineError(EngineErrorKind::Generic(format!(
            "Failed to load OCR detection model: {e}"
        )))
    })?;
    let recognition_model = rten::Model::load_file(&rec_path).map_err(|e| {
        PdfError::EngineError(EngineErrorKind::Generic(format!(
            "Failed to load OCR recognition model: {e}"
        )))
    })?;

    let engine = ocrs::OcrEngine::new(ocrs::OcrEngineParams {
        detection_model: Some(detection_model),
        recognition_model: Some(recognition_model),
        ..Default::default()
    })
    .map_err(|e| {
        PdfError::EngineError(EngineErrorKind::Generic(format!(
            "Failed to initialize OCR engine: {e}"
        )))
    })?;

    let arc_engine = Arc::new(engine);
    if let Ok(mut guard) = OCR_ENGINE_CACHE.lock() {
        guard.insert(script, arc_engine.clone());
    }

    Ok(arc_engine)
}

impl DocumentStore {
    pub fn new(cache: SharedRenderCache) -> Self {
        Self {
            documents: HashMap::new(),
            paths: HashMap::new(),
            passwords: HashMap::new(),
            render_cache: cache,
            oc_configs: HashMap::new(),
            oc_groups: HashMap::new(),
            oc_visibility: HashMap::new(),
            image_caches: HashMap::new(),
            image_cache_bytes: DEFAULT_IMAGE_CACHE_BYTES,
            loaded_generations: HashMap::new(),
        }
    }

    /// Which on-disk revision of `doc_id` this worker currently holds.
    pub fn loaded_generation(&self, doc_id: DocumentId) -> Option<u64> {
        self.loaded_generations.get(&doc_id).copied()
    }

    pub fn set_loaded_generation(&mut self, doc_id: DocumentId, generation: u64) {
        self.loaded_generations.insert(doc_id, generation);
    }

    pub fn clear_loaded_generation(&mut self, doc_id: DocumentId) {
        self.loaded_generations.remove(&doc_id);
    }

    /// The aggregate decoded-image budget applied to every interpreter this
    /// store runs. Exposed for tests and diagnostics.
    pub fn image_cache_bytes(&self) -> u64 {
        self.image_cache_bytes
    }

    /// Decoded-image bytes currently retained for `doc_id`.
    ///
    /// The whole point of `image_cache_bytes` is that this stays bounded for
    /// the life of the document rather than growing with every page scrolled
    /// past, so it is worth being able to assert on directly.
    pub fn image_cache_bytes_used(&self, doc_id: DocumentId) -> u64 {
        self.image_caches
            .get(&doc_id)
            .map_or(0, ImageCache::bytes_used)
    }

    pub fn has_document(&self, doc_id: DocumentId) -> bool {
        self.documents.contains_key(&doc_id)
    }

    /// Size and `/Rotate` of every page in an already-open document.
    ///
    /// Used after a reload so the view can resize its cards to the new file
    /// (a delete or rotate changes both the page count and the extents) without
    /// re-running the whole open pipeline.
    pub fn page_geometry(&self, doc_id: DocumentId) -> PdfResult<Vec<crate::models::PageInfo>> {
        let doc = self
            .documents
            .get(&doc_id)
            .ok_or(PdfError::EngineError(EngineErrorKind::DocumentNotFound))?;
        let count = doc.page_count();
        let mut pages = Vec::with_capacity(count);
        for i in 0..count {
            match doc.page(i) {
                Ok(page) => {
                    let rect = page.effective_box();
                    let unit = page.user_unit as f32;
                    pages.push(crate::models::PageInfo {
                        width: rect.width() as f32 * unit,
                        height: rect.height() as f32 * unit,
                        rotation: normalize_rotation(page.rotate),
                    });
                }
                Err(_) => pages.push(crate::models::PageInfo {
                    width: 0.0,
                    height: 0.0,
                    rotation: 0,
                }),
            }
        }
        Ok(pages)
    }

    pub fn open_document(
        &mut self,
        path: &str,
        password: Option<&str>,
        doc_id: DocumentId,
    ) -> PdfResult<crate::models::OpenResult> {
        let metadata = std::fs::metadata(path).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        if metadata.len() > 512 * 1024 * 1024 {
            return Err(PdfError::OpenFailed(
                "PDF file exceeds 512 MB size limit".into(),
            ));
        }
        let data = read_input_capped(path)?;
        // Harden against malformed/malicious PDFs by capping stream and object counts.
        let limits = ParseLimits {
            max_stream_bytes: 256 * 1024 * 1024, // 256 MB per stream
            // `ParseLimits::default()` allows a 1 GiB decoded-image budget and
            // 256 MB of fonts. This document is kept open for the whole session,
            // so those defaults dominate resident memory for small inputs.
            max_image_cache_bytes: DEFAULT_IMAGE_CACHE_BYTES,
            max_font_cache_bytes: 32 * 1024 * 1024,
            ..ParseLimits::default()
        };
        // Attach a system-enabled FontProvider so non-embedded fonts can be
        // substituted and variable/multi-script glyphs resolve against the
        // installed fonts (zpdf 0.15+). The scan of system fonts is cached
        // globally inside zpdf, so this is cheap after the first document.
        let font_provider = std::sync::Arc::new(zpdf::FontProvider::new(
            true,
            zpdf::FontProviderLimits::default(),
        ));
        let doc = match PdfDocument::open_with_password_and_limits(
            data,
            password.unwrap_or("").as_bytes(),
            limits,
        ) {
            Ok(mut doc) => {
                doc.set_font_provider(font_provider);
                doc
            }
            Err(zpdf::Error::WrongPassword | zpdf::Error::PasswordRequired) => {
                return Err(PdfError::PasswordRequired);
            }
            Err(e) => {
                return Err(PdfError::OpenFailed(e.to_string()));
            }
        };

        let page_count = doc.page_count();
        let mut heights = Vec::with_capacity(page_count);
        let mut widths = Vec::with_capacity(page_count);
        let mut rotations = Vec::with_capacity(page_count);
        let mut max_width = 0.0;

        for i in 0..page_count {
            if let Ok(page) = doc.page(i) {
                let rect = page.effective_box();
                let user_unit = page.user_unit as f32;
                let w = (rect.width() as f32) * user_unit;
                let h = (rect.height() as f32) * user_unit;
                // Report the **unrotated** extents, and the page's own `/Rotate`
                // alongside them. The canvas applies the rotation itself — it
                // needs `page_rotations` for that — and the overlay transform is
                // only correct when `page_width`/`page_height` are unrotated.
                // Reporting the *laid-out* extents here (as this used to) made
                // the canvas treat a rotated document as already turned, so
                // annotations, search hits and selections were all placed as if
                // the page were upright.
                let rotated = normalize_rotation(page.rotate);
                rotations.push(rotated);
                heights.push(h);
                widths.push(w);
                if w > max_width {
                    max_width = w;
                }
            } else {
                heights.push(0.0);
                widths.push(0.0);
                rotations.push(0);
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
        // Page label styles are not publicly exposed by zpdf's PageLabels API.
        // The label() method already returns the formatted string for each page.
        let is_encrypted = doc.is_encrypted();
        let system_anchors = Self::cached_system_trust_anchors();
        let signatures = Self::extract_signatures_internal(&doc, system_anchors);

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
            // Record the full group set so a later visibility toggle can be
            // layered over this document's real states instead of replacing
            // them. Derived from `layers`, which already walks `/OCGs`.
            self.oc_groups.insert(
                doc_id,
                layers
                    .iter()
                    .map(|l| zpdf::ObjectId(l.object_id.0, l.object_id.1))
                    .collect(),
            );
        }

        // Feature 4: Extract geospatial annotation metadata
        let geo_annotations = Self::extract_geo_annotations(&doc);
        // Feature 5: Extract CMYK/ICC color profile info
        let color_profile = Self::extract_color_profile(&doc);

        self.documents.insert(doc_id, doc);
        self.paths.insert(doc_id, path.to_string());
        if let Some(pw) = password {
            self.passwords.insert(doc_id, pw.to_string());
        }
        self.image_caches.insert(doc_id, ImageCache::new());

        Ok(crate::models::OpenResult {
            id: doc_id,
            page_count,
            page_heights: heights,
            page_widths: widths,
            max_width,
            page_rotations: rotations,
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

    /// Repoint a document at the file it should be read from and written to.
    ///
    /// [`Self::save_annotations`] and [`Self::apply_redactions`] take their
    /// *source* bytes from `paths` while writing to an explicit `output_path`.
    /// A successful Save-As must therefore move this map too, not only the
    /// worker's shared path map: otherwise the next Ctrl+S reads the original
    /// file and writes it over the Save-As target, silently discarding whatever
    /// that target held.
    pub fn set_document_path(&mut self, doc_id: DocumentId, path: String) {
        self.paths.insert(doc_id, path);
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

    /// Extract reading-order text using zpdf's `struct_ordered_text`, which
    /// follows the PDF tagged-structure tree. Falls back to plain
    /// `spans_to_text` for untagged documents.
    ///
    /// The returned flag says which path produced the text — a caller must not
    /// present the fallback as if the document were tagged.
    pub fn extract_accessible_text(
        &self,
        doc_id: DocumentId,
        page_num: usize,
    ) -> PdfResult<crate::models::AccessibleTextResult> {
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
                .with_page_user_unit(page.user_unit)
                .with_image_cache_limit(self.image_cache_bytes)
                .with_fonts(&mut fonts)
                .with_document(doc.file(), &page.resources)
                .with_images(&mut images)
                .with_text_sink(&mut spans);
            let _ = interp.interpret(&content);
        }

        // Try struct_ordered_text first (uses /MCID for tagged-PDF reading order).
        // Fall back to plain spans_to_text for untagged documents.
        let reading_order_applied = doc.struct_tree().is_some();
        let text = if let Some(ref tree) = doc.struct_tree() {
            struct_ordered_text(&spans, page_num, tree)
        } else {
            spans_to_text(spans, 2.0)
        };
        Ok(crate::models::AccessibleTextResult {
            page: page_num,
            char_count: text.chars().count(),
            text,
            reading_order_applied,
        })
    }

    /// Resolve every link annotation through zpdf's destination parser.
    ///
    /// Unlike [`Self::extract_links_internal`] — which drops everything except
    /// a bare page number — this keeps the declared view so a click can honour
    /// `XYZ left top zoom` instead of just landing at the top of the page.
    pub fn extract_link_destinations(
        &self,
        doc_id: DocumentId,
    ) -> PdfResult<Vec<crate::models::LinkDestinationInfo>> {
        let Some(doc) = self.documents.get(&doc_id) else {
            return Err(PdfError::EngineError(EngineErrorKind::DocumentNotFound));
        };
        let mut results = Vec::new();
        for page_idx in 0..doc.page_count() {
            let Ok(page) = doc.page(page_idx) else {
                continue;
            };
            // The page box is only needed to normalise the annotation rect.
            let eff_box = page.effective_box();
            let annots = doc.page_annotations(&page);
            for annot in annots {
                if annot.subtype != "Link" {
                    continue;
                }
                let bounds = (
                    annot.rect.x0 as f32 - eff_box.x0 as f32,
                    (eff_box.y1 - annot.rect.y1) as f32,
                    (annot.rect.x1 - annot.rect.x0) as f32,
                    (annot.rect.y1 - annot.rect.y0) as f32,
                );
                match annot.uri.clone() {
                    Some(url) => results.push(crate::models::LinkDestinationInfo {
                        page: page_idx,
                        kind: crate::models::LinkKind::External,
                        url: Some(url),
                        target_page: None,
                        view: None,
                        bounds,
                    }),
                    None => {
                        let (target_page, view) = annot.dest.as_ref().map_or((None, None), |d| {
                            (
                                d.page,
                                Some(crate::models::DestinationView::from_dest_view(
                                    &d.view,
                                    d.page.is_some(),
                                )),
                            )
                        });
                        results.push(crate::models::LinkDestinationInfo {
                            page: page_idx,
                            kind: crate::models::LinkKind::Internal,
                            url: None,
                            target_page,
                            view: view.map(|v| v.label().to_string()),
                            bounds,
                        });
                    }
                }
            }
        }
        Ok(results)
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
        let system_anchors = Self::cached_system_trust_anchors();
        let signatures = Self::extract_signatures_internal(doc, system_anchors);

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
            self.oc_groups.insert(
                doc_id,
                layers
                    .iter()
                    .map(|l| zpdf::ObjectId(l.object_id.0, l.object_id.1))
                    .collect(),
            );
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
        self.loaded_generations.remove(&doc_id);
        self.paths.remove(&doc_id);
        self.passwords.remove(&doc_id);
        self.oc_configs.remove(&doc_id);
        self.oc_groups.remove(&doc_id);
        self.oc_visibility.remove(&doc_id);
        self.image_caches.remove(&doc_id);
        self.render_cache.remove_document(doc_id);
    }

    pub fn cached_system_trust_anchors() -> &'static [zpdf::trust::TrustAnchor] {
        static ANCHORS: std::sync::OnceLock<Vec<zpdf::trust::TrustAnchor>> =
            std::sync::OnceLock::new();
        ANCHORS.get_or_init(Self::load_system_trust_anchors)
    }

    /// Load trusted root certificates from the OS (e.g. Windows ROOT & CA stores)
    pub fn load_system_trust_anchors() -> Vec<zpdf::trust::TrustAnchor> {
        #[cfg(windows)]
        {
            use windows::Win32::Security::Cryptography::{
                CertCloseStore, CertEnumCertificatesInStore, CertFreeCertificateContext,
                CertOpenSystemStoreW,
            };
            use windows::core::w;

            let mut anchors = Vec::new();
            for store_name in [w!("ROOT"), w!("CA")] {
                unsafe {
                    if let Ok(store) = CertOpenSystemStoreW(None, store_name) {
                        let mut p_ctx = CertEnumCertificatesInStore(store, None);
                        while !p_ctx.is_null() {
                            // A truncated store entry can carry a null
                            // `pbCertEncoded`; `slice::from_raw_parts` requires a
                            // non-null, aligned pointer even for a zero-length
                            // slice, so skip it rather than invoke UB.
                            let ptr = (*p_ctx).pbCertEncoded;
                            let len = (*p_ctx).cbCertEncoded as usize;
                            if !ptr.is_null() && len > 0 {
                                let cert_slice = std::slice::from_raw_parts(ptr, len);
                                let mut parsed = zpdf::trust::parse_trust_anchors(cert_slice);
                                anchors.append(&mut parsed);
                            }
                            // `CertEnumCertificatesInStore` frees the
                            // `CERT_CONTEXT` handed to it as
                            // `pPrevCertificateContext` and returns the next
                            // one, so advancing the cursor *is* the release.
                            // Freeing it as well is a double free: the
                            // enumerator then hands back a dangling pointer
                            // that the next iteration dereferences.
                            p_ctx = CertEnumCertificatesInStore(store, Some(p_ctx));
                        }
                        // The cursor returned by the final call is not consumed
                        // by any later call, so it is the one that must be
                        // released here — one leaked context per store per call
                        // before this.
                        let _ = CertFreeCertificateContext(Some(p_ctx as *const _));
                        let _ = CertCloseStore(Some(store), 0);
                    }
                }
            }
            anchors
        }
        #[cfg(not(windows))]
        {
            Vec::new()
        }
    }

    /// Evaluate X.509 certificate chain trust against given anchors
    pub fn evaluate_signature_trust(
        cms_blob: Option<&[u8]>,
        anchors: &[zpdf::trust::TrustAnchor],
    ) -> (bool, Option<String>, Vec<String>) {
        if let Some(cms) = cms_blob {
            let status = zpdf::trust::verify_certificate_chain(cms, anchors, None);
            match status {
                zpdf::trust::ChainStatus::Trusted(cns) => {
                    let chain_str = cns.join(" → ");
                    (true, Some(format!("Trusted Root CA ({chain_str})")), cns)
                }
                zpdf::trust::ChainStatus::Untrusted(msg) => (
                    false,
                    Some(format!("Untrusted Root / Self-Signed: {msg}")),
                    Vec::new(),
                ),
                zpdf::trust::ChainStatus::Unsupported(msg) => (
                    false,
                    Some(format!("Unsupported Certificate: {msg}")),
                    Vec::new(),
                ),
            }
        } else {
            (false, Some("No CMS contents".to_string()), Vec::new())
        }
    }

    /// Extract signature information and evaluate trust chains
    fn extract_signatures_internal(
        doc: &PdfDocument,
        anchors: &[zpdf::trust::TrustAnchor],
    ) -> Vec<crate::models::SignatureInfo> {
        doc.signatures()
            .into_iter()
            .map(|sig| {
                let (is_trusted, trust_status, cert_chain) =
                    Self::evaluate_signature_trust(sig.cms_blob.as_deref(), anchors);
                let revocation_status = Some(format!("{:?}", sig.revocation));
                let byte_range_intact = sig.coverage.covers_whole_document;
                crate::models::SignatureInfo {
                    field_name: sig.field_name,
                    signer_name: sig.signer_common_name.or(sig.name),
                    signing_time: sig.signing_time,
                    location: sig.location,
                    reason: sig.reason,
                    digest_verified: matches!(sig.digest, zpdf::DigestStatus::Verified),
                    crypto_valid: matches!(sig.crypto, zpdf::CryptoStatus::Valid),
                    is_trusted,
                    trust_status,
                    cert_chain,
                    revocation_status,
                    byte_range_intact,
                }
            })
            .collect()
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
            if let zpdf::PdfObject::Ref(r) = item
                && let Ok(ocg_obj) = doc.file().resolve(*r)
                && let Ok(ocg_dict) = ocg_obj.as_dict()
            {
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
        layers
    }

    pub fn toggle_layer(&mut self, doc_id: DocumentId, object_id: (u32, u16), visible: bool) {
        if self.oc_configs.contains_key(&doc_id) {
            let id = zpdf::ObjectId(object_id.0, object_id.1);
            self.oc_visibility
                .entry(doc_id)
                .or_default()
                .insert(id, visible);

            // Invalidate the render cache for this document across all workers.
            self.render_cache.invalidate_document(doc_id);
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
    ///
    /// Returns an empty vec only when the document genuinely has none. A file
    /// that cannot be parsed is an error, not an empty list: the previous code
    /// flattened every failure into `Ok(vec![])`, so a password-protected
    /// document (which `lopdf` cannot open) always looked like "your highlights
    /// are gone" — and the caller would then save that empty list back over the
    /// file.
    #[allow(clippy::many_single_char_names, clippy::similar_names)]
    pub fn load_annotations(&self, path: &str) -> PdfResult<Vec<Annotation>> {
        // Capped like every other entry point: `Document::load` read the whole
        // file with no size limit, so a multi-GB input OOM'd the worker before
        // any check could refuse it.
        let data = read_input_capped(path)?;
        let lopdf_doc = Document::load_mem(&data)
            .map_err(|e| PdfError::OpenFailed(format!("cannot read annotations: {e}")))?;

        let mut annotations = Vec::new();
        let pages = lopdf_doc.get_pages();

        for (&page_num, &page_id) in &pages {
            let page_idx = (page_num as usize).saturating_sub(1);

            // Convert PDF bottom-left-origin coordinates into the viewport's
            // top-left-origin space. The correct divisor is the page box's
            // **y1**, not its height: `y_top = y1 - y_pdf - h`.
            //
            // These are different whenever the box does not start at y=0, which
            // is the norm for scanned pages (`MediaBox [0 100 612 892]`) and for
            // anything with a `CropBox` (`[50 50 562 742]`, height 692 vs
            // MediaBox[3] 792). Using height here meant every annotation drifted
            // further down the page on each open/save cycle, because the write
            // path in `save_annotations` uses the true height — the two
            // disagreed, so each round trip added `y0`.
            let (page_x0, _, _, page_y1) =
                effective_page_box(&lopdf_doc, page_id).unwrap_or((0.0, 0.0, 612.0, 792.0));

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

                // Only this app's own annotations are loaded. A highlight another
                // reader wrote carries no `/NM` marker, so `save_annotations`
                // does not strip it — but it *is* in the list handed back to the
                // UI, and the save path appends that whole list. Loading a
                // foreign annotation therefore made every Ctrl+S append one more
                // copy of it. The marker exists precisely to identify this app's
                // own annotations; honouring it on read as well as on write is
                // what makes the annotation set round-trip.
                //
                // Consequence: annotations written by other applications are no
                // longer shown in the sidebar, because the app cannot save them
                // back without duplicating them.
                if !is_pdfbull_annotation_lopdf(dict) {
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
                            Object::Real(v) => *v,
                            Object::Integer(v) => *v as f32,
                            _ => 0.0_f32,
                        };
                        (to_f32(a), to_f32(b), to_f32(c), to_f32(d))
                    }
                    _ => continue,
                };

                // Convert from PDF bottom-left origin to screen top-left origin.
                // `x` is taken relative to the box's x0 so a `CropBox` with a
                // non-zero left edge does not shift every annotation sideways.
                let x = pdf_x0 - page_x0;
                let w = (pdf_x1 - pdf_x0).abs();
                let h = (pdf_y1 - pdf_y0).abs();
                let y = page_y1 - pdf_y1;

                // Parse color
                let color_str = dict
                    .get(b"C")
                    .ok()
                    .and_then(|o| o.as_array().ok())
                    .cloned()
                    .and_then(|arr| {
                        let to_f32 = |o: &Object| match o {
                            Object::Real(v) => *v,
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
                        Object::Real(v) => Some(*v),
                        Object::Integer(v) => Some(*v as f32),
                        _ => None,
                    })
                    .unwrap_or(2.0_f32);

                let style = match subtype.as_str() {
                    "Highlight" => Some(AnnotationStyle::Highlight { color: color_str }),
                    "Underline" => Some(AnnotationStyle::Underline { color: color_str }),
                    "StrikeOut" => Some(AnnotationStyle::Strikeout { color: color_str }),
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
                    // A `Line` annotation also carries `/L [x1 y1 x2 y2]`, which
                    // keeps the direction the normalised `Rect` throws away.
                    // Without reading it, a line drawn bottom-left to
                    // top-right came back as the opposite diagonal.
                    let is_line = matches!(
                        style,
                        AnnotationStyle::Line { .. } | AnnotationStyle::Arrow { .. }
                    );
                    let endpoints = is_line
                        .then(|| {
                            let arr = dict.get(b"L").ok()?.as_array().ok()?;
                            let to_f32 = |o: &Object| match o {
                                Object::Real(v) => Some(*v),
                                Object::Integer(v) => Some(*v as f32),
                                _ => None,
                            };
                            match arr.as_slice() {
                                [a, b, c, d] => {
                                    let (x1, y1) = (to_f32(a)?, to_f32(b)?);
                                    let (x2, y2) = (to_f32(c)?, to_f32(d)?);
                                    Some((
                                        x1 - page_x0,
                                        page_y1 - y1,
                                        (x2 - page_x0) - (x1 - page_x0),
                                        (page_y1 - y2) - (page_y1 - y1),
                                    ))
                                }
                                _ => None,
                            }
                        })
                        .flatten();
                    let (ann_x, ann_y, ann_w, ann_h) = endpoints.unwrap_or((x, y, w, h));
                    annotations.push(Annotation {
                        id: crate::models::next_annotation_id(),
                        page: page_idx,
                        x: ann_x,
                        y: ann_y,
                        width: ann_w,
                        height: ann_h,
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
            rotation: options.rotation,
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
            // Thumbnails are always colour-true, so they must not share a cache
            // entry with a filtered full-page render.
            filter: if is_thumbnail {
                RenderFilter::None
            } else {
                options.filter
            },
        };

        // Capture the cache generation *before* rendering. If the document is
        // invalidated while this render is in flight (another worker processing a
        // layer toggle, or this document being closed), `put_if_current` below
        // will decline to store the now-stale bitmap.
        let render_generation = self.render_cache.generation(doc_id);

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
        let images = self.image_caches.entry(doc_id).or_default();
        let content = doc
            .page_content_bytes(&page)
            .map_err(|e| PdfError::RenderFailed(e.to_string()))?;
        let page_rect = page.effective_box();

        // Incorporate custom option rotation into the display list rotation.
        //
        // `with_image_cache_limit` is mandatory here: `ContentInterpreter::new`
        // inherits `ParseLimits::default()`, whose `max_image_cache_bytes` is
        // 1 GiB, and `images` below is the document-lifetime cache. Without
        // this line the store silently retains up to a gigabyte of decoded
        // pixels per open document.
        let mut interp = ContentInterpreter::new(page_rect)
            .with_image_cache_limit(self.image_cache_bytes)
            .with_page_rotation(page.rotate + options.rotation)
            .with_page_user_unit(page.user_unit)
            .with_fonts(&mut fonts)
            .with_document(doc.file(), &page.resources)
            .with_images(images);

        // Ask the CPU backend what scale it will actually rasterize this page
        // at — `fit_raster` may reduce it to stay inside the page-pixel budget
        // — and pre-filter every image the backend would box-filter on draw
        // (an axis below 0.5x of its samples) down to that footprint. The
        // backend applies the same rule and filter on draw, so the pixels are
        // unchanged; what changes is that a scan is no longer retained in the
        // document-lifetime cache above at its full decoded size when it is
        // drawn thumbnail-small. See `DEFAULT_IMAGE_CACHE_BYTES` for why that
        // retention is the binding constraint here.
        //
        // A bare `CpuRenderer::new()` is the correct oracle for the scale:
        // `raster_scale` reads only the page-pixel budget, which `with_fonts`
        // and `with_images` below do not affect, and the budget is derived from
        // the pixel *count*, so passing the unrotated box is equivalent to what
        // the rotated display list yields. If a `with_limits` call is ever added
        // to the renderer below, it must be mirrored here or the two scale
        // calculations will diverge.
        if let Some(scale) = CpuRenderer::new().raster_scale(page_rect, options.scale) {
            interp = interp.with_image_downscale(scale);
        }

        // Build optional-content config with user visibility overrides applied.
        let oc_for_render: Option<zpdf::OcConfig> = self.oc_configs.get(&doc_id).map(|base_oc| {
            if let Some(overrides) = self.oc_visibility.get(&doc_id)
                && !overrides.is_empty()
            {
                apply_oc_overrides(
                    base_oc,
                    overrides,
                    self.oc_groups
                        .get(&doc_id)
                        .map_or(&[][..], |v| v.as_slice()),
                )
            } else {
                base_oc.clone()
            }
        });

        if let Some(ref oc) = oc_for_render {
            interp = interp.with_optional_content(oc);
        }

        let display_list = interp.interpret(&content);

        let mut renderer = CpuRenderer::new()
            .with_fonts(&fonts)
            .with_images(images)
            .with_stage_timing(true);
        let page_img = renderer
            .render_display_list(&display_list, options.scale)
            .map_err(|e| PdfError::RenderFailed(e.to_string()))?;
        if let Some(stats) = renderer.stage_stats() {
            tracing::debug!(
                page = page_num,
                total_ms = stats.total_ns / 1_000_000,
                glyphs = stats.glyphs,
                glyph_ms = stats.glyph_ns() / 1_000_000,
                fills = stats.fills,
                fill_ms = stats.fill_ns / 1_000_000,
                strokes = stats.strokes,
                stroke_ms = stats.stroke_ns / 1_000_000,
                images = stats.images,
                image_ms = stats.image_ns / 1_000_000,
                masks = stats.soft_mask_planes,
                mask_ms = stats.soft_mask_ns / 1_000_000,
                "Page render stage diagnostics"
            );
        }
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

        // Store only if the document has not changed since this render started.
        // The caller still gets its bitmap either way -- it is correct for the
        // state it rendered -- it just must not become the cached answer for
        // subsequent requests.
        self.render_cache
            .put_if_current(cache_key, base.clone(), render_generation);

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
                .with_page_user_unit(page.user_unit)
                .with_image_cache_limit(self.image_cache_bytes)
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
                .with_page_user_unit(page.user_unit)
                .with_image_cache_limit(self.image_cache_bytes)
                .with_fonts(&mut fonts)
                .with_document(doc.file(), &page.resources)
                .with_images(&mut images)
                .with_text_sink(&mut spans);
            let _ = interp.interpret(&content);
        }

        // `search` already used the box's y1 here; the text layer used the box
        // *height*, which disagreed whenever the box does not start at y=0. The
        // `.max(0.0)` then clamped every span in the lower half of the page to
        // y=0, collapsing text selection onto the top edge.
        let eff_box = page.effective_box();
        let mut text_items = Vec::new();
        for span in spans {
            if span.text.trim().is_empty() {
                continue;
            }
            let char_count = span.text.chars().count();
            let total_adv = if span.advance.abs() > 0.01 {
                span.advance.abs() as f32
            } else {
                span.size * 0.5 * char_count.max(1) as f32
            };
            let char_step = if char_count > 0 {
                total_adv / char_count as f32
            } else {
                0.0
            };

            let base_x = (span.x - eff_box.x0) as f32;
            let base_y = (eff_box.y1 - span.y - span.size as f64) as f32;

            if span.text.contains(char::is_whitespace) {
                let mut current_word = String::new();
                let mut word_start_idx = 0;
                for (idx, ch) in span.text.chars().enumerate() {
                    if ch.is_whitespace() {
                        if !current_word.is_empty() {
                            let word_chars = current_word.chars().count();
                            let word_x = base_x + char_step * word_start_idx as f32;
                            let word_w = char_step * word_chars as f32;
                            text_items.push(crate::models::TextItem {
                                text: current_word,
                                x: word_x,
                                y: base_y,
                                width: word_w,
                                height: span.size,
                            });
                            current_word = String::new();
                        }
                    } else {
                        if current_word.is_empty() {
                            word_start_idx = idx;
                        }
                        current_word.push(ch);
                    }
                }
                if !current_word.is_empty() {
                    let word_chars = current_word.chars().count();
                    let word_x = base_x + char_step * word_start_idx as f32;
                    let word_w = char_step * word_chars as f32;
                    text_items.push(crate::models::TextItem {
                        text: current_word,
                        x: word_x,
                        y: base_y,
                        width: word_w,
                        height: span.size,
                    });
                }
            } else {
                text_items.push(crate::models::TextItem {
                    text: span.text,
                    x: base_x,
                    y: base_y,
                    width: total_adv,
                    height: span.size,
                });
            }
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
        let mut rules = Vec::new();
        {
            let interp = ContentInterpreter::new(page.effective_box())
                .with_page_user_unit(page.user_unit)
                .with_image_cache_limit(self.image_cache_bytes)
                .with_fonts(&mut fonts)
                .with_document(doc.file(), &page.resources)
                .with_images(&mut images)
                .with_text_sink(&mut spans)
                .with_rule_sink(&mut rules);
            let _ = interp.interpret(&content);
        }

        let tables = detect_tables_with_rules(&spans, &rules);
        let eff_box = page.effective_box();

        let detected = tables
            .into_iter()
            .map(|t| {
                let (x0, y0, x1, y1) = t.bbox();
                // Coordinate conversion to y-down layout space, relative to the
                // page box origin: x = x0 - box.x0, y = box.y1 - y1.
                let bbox = (
                    (x0 - eff_box.x0) as f32,
                    (eff_box.y1 - y1) as f32,
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

        let data = read_input_capped(&pdf_path)?;
        let pass = self
            .passwords
            .get(&doc_id)
            .map(|s| s.as_bytes())
            .unwrap_or(b"");
        let mut writer = IncrementalWriter::new_with_password(data, pass)
            .map_err(|e| PdfError::OpenFailed(e.to_string()))?;

        // Idempotency: drop every annotation a *previous* save wrote before
        // appending the current set. Without this, each Ctrl+S re-reads the
        // already-annotated file and appends the same annotations again, so
        // highlights silently multiply every time the user saves.
        let page_count = writer.document().page_count();
        let mut stale: Vec<(usize, zpdf::ObjectId)> = Vec::new();
        for page_idx in 0..page_count {
            let Ok(page) = writer.document().page(page_idx) else {
                continue;
            };
            for annot_id in &page.annots {
                match writer.resolve_current(*annot_id) {
                    Ok(obj) => {
                        if is_pdfbull_annotation(&obj) {
                            stale.push((page_idx, *annot_id));
                        }
                    }
                    // A `/Annots` entry that cannot be resolved is not evidence
                    // that the annotation is someone else's. Treating it as
                    // "not ours" classified it as foreign, so it was never
                    // deleted -- and the new set was appended on top. That is
                    // precisely the duplication the delete loop below exists to
                    // prevent, and it would be reported as a successful save.
                    //
                    // The delete path already refuses on failure; make the
                    // *detection* path equally strict rather than silently
                    // guessing.
                    Err(e) => {
                        return Err(PdfError::EngineError(EngineErrorKind::Generic(format!(
                            "page {} references an annotation ({}) that could not be \
                                 read, so the existing annotations cannot be replaced safely. \
                                 The file was not modified. ({e})",
                            page_idx + 1,
                            annot_id.0
                        ))));
                    }
                }
            }
        }
        // This loop exists precisely to stop annotations multiplying on every save. If
        // a delete fails, the old annotation survives *and* the new set is
        // appended — which is exactly the duplication this loop prevents — and
        // the save is then reported as successful. Refuse rather than write a
        // file whose annotation set we cannot vouch for.
        for (page_idx, annot_id) in stale {
            writer.delete_annotation(page_idx, annot_id).map_err(|e| {
                PdfError::EngineError(EngineErrorKind::Generic(format!(
                    "could not remove the previous annotation on page {} while saving: {e}",
                    page_idx + 1
                )))
            })?;
        }

        for ann in annotations {
            // Skip redact annotations — those are applied via apply_redactions().
            if matches!(&ann.style, AnnotationStyle::Redact { .. }) {
                continue;
            }

            // The viewport stores annotations in top-left-origin space
            // relative to the page box's own origin, so converting back means
            // using the box's `y1` and adding its `x0` — mirroring
            // `load_annotations` exactly. Using the box *height* here (rather
            // than y1) is what made positions drift by `y0` on every
            // open/save cycle for any page whose box does not start at y=0.
            // No Letter-size fallback: substituting 612x792 for a page whose
            // real box is unknown puts the annotation in the wrong place, and
            // for a redaction that means burning the wrong region. Refuse
            // instead of guessing.
            let page_box = self
                .documents
                .get(&doc_id)
                .and_then(|d| d.page(ann.page).ok())
                .map(|p| p.effective_box())
                .ok_or_else(|| {
                    PdfError::EngineError(EngineErrorKind::Generic(format!(
                        "cannot place annotation: page {} of the document is not loaded",
                        ann.page + 1
                    )))
                })?;

            let pdf_x = ann.x as f64 + page_box.x0;
            let pdf_w = ann.width as f64;
            let pdf_h = ann.height as f64;
            // Convert from screen top-left to PDF bottom-left origin.
            let pdf_y = page_box.y1 - ann.y as f64 - pdf_h;

            let rect = ZpdfRect {
                x0: pdf_x,
                y0: pdf_y,
                x1: pdf_x + pdf_w,
                y1: pdf_y + pdf_h,
            };

            let is_arrow = matches!(&ann.style, AnnotationStyle::Arrow { .. });

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
                AnnotationStyle::Underline { color } => {
                    let (r, g, b) = hex_to_rgb(color);
                    AnnotationSpec::markup_from_rects(
                        MarkupKind::Underline,
                        &[rect],
                        (r as f64, g as f64, b as f64),
                        None,
                    )
                }
                AnnotationStyle::Strikeout { color } => {
                    let (r, g, b) = hex_to_rgb(color);
                    AnnotationSpec::markup_from_rects(
                        MarkupKind::StrikeOut,
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
                AnnotationStyle::Line { color, thickness }
                | AnnotationStyle::Arrow { color, thickness } => {
                    let (r, g, b) = hex_to_rgb(color);
                    // Preserve true start and end endpoints in PDF space regardless of draw direction
                    let x1 = ann.x as f64 + page_box.x0;
                    let y1 = page_box.y1 - ann.y as f64;
                    let x2 = (ann.x + ann.width) as f64 + page_box.x0;
                    let y2 = page_box.y1 - (ann.y + ann.height) as f64;
                    AnnotationSpec::Line {
                        x1,
                        y1,
                        x2,
                        y2,
                        color: (r as f64, g as f64, b as f64),
                        width: *thickness as f64,
                    }
                }
                AnnotationStyle::Redact { .. } => continue, // filtered above
            };

            let annot_id = writer
                .add_annotation(ann.page, &spec)
                .map_err(|e| PdfError::IoError(e.to_string()))?;

            // Stamp the marker so the next save can replace this annotation
            // rather than duplicate it, and record the arrow head in /LE so
            // load_annotations can restore Arrow annotations (not plain Lines).
            if let Ok(obj) = writer.resolve_current(annot_id)
                && let Ok(mut dict) = obj.as_dict().cloned()
            {
                dict.insert(
                    zpdf::PdfName::new("NM"),
                    zpdf::PdfObject::String(zpdf::PdfString::new(
                        format!("{PDFBULL_ANNOT_MARKER}{}", ann.id).into_bytes(),
                    )),
                );
                if is_arrow {
                    dict.insert(
                        zpdf::PdfName::new("LE"),
                        zpdf::PdfObject::Array(vec![
                            zpdf::PdfObject::Name(zpdf::PdfName::new("None")),
                            zpdf::PdfObject::Name(zpdf::PdfName::new("Arrow")),
                        ]),
                    );
                }
                writer.overwrite_object(annot_id, zpdf::PdfObject::Dict(dict));
            }
        }

        let out_bytes = {
            let mut buf = std::io::Cursor::new(Vec::new());
            writer
                .write(&mut buf)
                .map_err(|e| PdfError::IoError(e.to_string()))?;
            buf.into_inner()
        };

        // `None` means the caller could not resolve the document's own path
        // (it has been closed mid-session). Fall back to a sidecar file rather
        // than silently clobbering whatever `pdf_path` points at.
        let final_path = match output_path {
            Some(p) => p,
            None => {
                let mut p = std::path::Path::new(&pdf_path).to_path_buf();
                let stem = p
                    .file_stem()
                    .map(|s| s.to_string_lossy())
                    .unwrap_or_default();
                p.set_file_name(format!("{stem}_annotated.pdf"));
                p.to_string_lossy().to_string()
            }
        };

        // Write via a temp file + rename so a failure part-way through never
        // leaves the user with a truncated (unopenable) PDF.
        crate::storage::atomic_write_bytes(std::path::Path::new(&final_path), &out_bytes)
            .map_err(|e| PdfError::IoError(e.to_string()))?;

        Ok(final_path)
    }

    pub fn export_page_as_image(
        &self,
        doc_id: DocumentId,
        page_num: usize,
        scale: f32,
    ) -> PdfResult<Vec<u8>> {
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
            .with_image_cache_limit(self.image_cache_bytes)
            .with_page_rotation(page.rotate)
            .with_page_user_unit(page.user_unit)
            .with_fonts(&mut fonts)
            .with_document(doc.file(), &page.resources)
            .with_images(&mut images)
            .interpret(&content);

        let mut renderer = CpuRenderer::new()
            .with_fonts(&fonts)
            .with_images(&images)
            .with_stage_timing(true);
        let page_img = renderer
            .render_display_list(&display_list, scale)
            .map_err(|e| PdfError::RenderFailed(e.to_string()))?;
        if let Some(stats) = renderer.stage_stats() {
            tracing::debug!(
                page = page_num,
                total_ms = stats.total_ns / 1_000_000,
                glyphs = stats.glyphs,
                "PNG export render stage diagnostics"
            );
        }
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

    pub fn search(&mut self, doc_id: DocumentId, query: &str) -> PdfResult<Vec<SearchResultItem>> {
        // An empty needle matches at every position: `str::find("")` is always
        // `Some(0)`, so the scan loop would return one hit per character of the
        // page text. The UI filters empty queries, but this is a public API.
        if query.is_empty() {
            return Ok(Vec::new());
        }
        let doc = self
            .documents
            .get(&doc_id)
            .ok_or(PdfError::EngineError(EngineErrorKind::DocumentNotFound))?;
        let total_pages = doc.page_count();

        let mut results = Vec::new();

        // Cap the result set. The UI truncates to `MAX_SEARCH_RESULTS` after the
        // engine has already built and shipped the whole vector, so an unbounded
        // search for a common substring in a long document materialised millions
        // of `SearchResultItem`s -- each owning a `String` -- inside a worker
        // before the cap could apply. Searching `"e"` in a 10,000-page document
        // was the realistic trigger. Stop early and say so.
        const MAX_RESULTS: usize = 5_000;
        let mut truncated = false;

        'pages: for page_idx in 0..total_pages {
            let Ok(page) = doc.page(page_idx) else {
                continue;
            };
            let mut fonts = doc.load_page_fonts(&page);
            // Share the document-lifetime image cache instead of building a fresh
            // one per page, per query. A local `ImageCache::new()` inside this
            // loop meant every keystroke re-ran `ContentInterpreter` over every
            // page and re-decoded every embedded image from scratch, while the
            // caches `render_page_internal` maintains went unused.
            let images = self.image_caches.entry(doc_id).or_default();
            let Ok(content) = doc.page_content_bytes(&page) else {
                continue;
            };

            let mut spans: Vec<zpdf::TextSpan> = Vec::new();
            let eff_box = page.effective_box();
            {
                let interp = ContentInterpreter::new(eff_box)
                    .with_page_user_unit(page.user_unit)
                    .with_image_cache_limit(self.image_cache_bytes)
                    .with_fonts(&mut fonts)
                    .with_document(doc.file(), &page.resources)
                    .with_images(images)
                    .with_text_sink(&mut spans);
                let _ = interp.interpret(&content);
            }

            // Use zpdf's built-in search_spans for accurate per-character
            // highlighting rects instead of the previous span-level approximation.
            let hits = search_spans(&spans, query, false);

            for hit in hits {
                if results.len() >= MAX_RESULTS {
                    truncated = true;
                    break 'pages;
                }

                // Transform the first rect from PDF user space (y-up) to the
                // app's top-down page coordinate system.
                let bounds = hit.bounds();
                let x_left = (bounds.x0 - eff_box.x0) as f32;
                let y_top_down = (eff_box.y1 - bounds.y1) as f32;
                let width = (bounds.x1 - bounds.x0) as f32;
                let height = (bounds.y1 - bounds.y0) as f32;

                results.push(SearchResultItem {
                    page_index: page_idx,
                    text: hit.line,
                    x: x_left,
                    y: y_top_down,
                    width,
                    height,
                });
            }
        }

        if truncated {
            tracing::info!(
                "search results capped at {MAX_RESULTS}; more matches exist in this document"
            );
        }

        Ok(results)
    }

    /// Update document metadata using zpdf_writer's `InfoUpdate`.
    #[allow(clippy::too_many_arguments)]
    pub fn update_metadata(
        &mut self,
        doc_id: DocumentId,
        title: Option<&str>,
        author: Option<&str>,
        subject: Option<&str>,
        keywords: Option<&str>,
        creator: Option<&str>,
        producer: Option<&str>,
    ) -> PdfResult<Vec<u8>> {
        use zpdf_writer::InfoUpdate;

        let info = InfoUpdate {
            title: title.map(|t| Some(t.to_string())),
            author: author.map(|a| Some(a.to_string())),
            subject: subject.map(|s| Some(s.to_string())),
            keywords: keywords.map(|k| Some(k.to_string())),
            creator: creator.map(|c| Some(c.to_string())),
            producer: producer.map(|p| Some(p.to_string())),
        };
        if info.is_empty() {
            return Ok(Vec::new());
        }

        // Build from the file **on disk**, not from `doc.file().data()`.
        //
        // The in-memory copy is whatever was loaded at open time, so a save
        // that happened after that (annotations, a previous metadata edit) is
        // absent from it. Writing these bytes back over the file therefore
        // silently discarded every change made since the document was opened —
        // most visibly, a highlight the user had just saved.
        let pdf_path = self
            .paths
            .get(&doc_id)
            .cloned()
            .ok_or(PdfError::EngineError(EngineErrorKind::DocumentPathNotFound))?;
        let data = read_input_capped(&pdf_path)?;
        let pass = self
            .passwords
            .get(&doc_id)
            .map(|s| s.as_bytes())
            .unwrap_or(b"");

        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let mut writer = IncrementalWriter::new_with_password(data, pass)
                .map_err(|e| PdfError::IoError(format!("metadata update failed: {e}")))?;
            writer
                .set_info(&info)
                .map_err(|e| PdfError::IoError(format!("metadata update failed: {e}")))?;
            writer
                .write(&mut buffer)
                .map_err(|e| PdfError::IoError(format!("metadata update failed: {e}")))?;
        }

        Ok(buffer.into_inner())
    }

    /// Extract a subset of pages into a new PDF using zpdf_writer's `extract_pages`.
    pub fn extract_pages_to_new(
        &mut self,
        doc_id: DocumentId,
        page_indices: &[usize],
    ) -> PdfResult<Vec<u8>> {
        // Same staleness as `update_metadata`: extract from the bytes that are
        // actually on disk so annotations saved after the document was opened
        // are carried into the extracted file instead of being dropped.
        let pdf_path = self
            .paths
            .get(&doc_id)
            .cloned()
            .ok_or(PdfError::EngineError(EngineErrorKind::DocumentPathNotFound))?;
        let data = read_input_capped(&pdf_path)?;
        let pass = self
            .passwords
            .get(&doc_id)
            .map(|s| s.as_bytes())
            .unwrap_or(b"");
        let file = zpdf::PdfFile::parse_with_password(data, pass)
            .map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        zpdf_writer::extract_pages(&file, page_indices)
            .map_err(|e| PdfError::IoError(format!("page extraction failed: {e}")))
    }

    /// Detect tables on a page using zpdf's plain `detect_tables` (without rules).
    pub fn detect_tables_plain_on_page(
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
                .with_page_user_unit(page.user_unit)
                .with_image_cache_limit(self.image_cache_bytes)
                .with_fonts(&mut fonts)
                .with_document(doc.file(), &page.resources)
                .with_images(&mut images)
                .with_text_sink(&mut spans);
            let _ = interp.interpret(&content);
        }

        let tables = detect_tables(&spans);
        let eff_box = page.effective_box();
        Ok(tables
            .into_iter()
            .map(|t| {
                // Compute bounding box from col_x/row_y arrays
                let min_x = t.col_x.first().copied().unwrap_or(0.0);
                let max_x = t.col_x.last().copied().unwrap_or(0.0);
                let max_y = t.row_y.first().copied().unwrap_or(0.0);
                let min_y = t.row_y.last().copied().unwrap_or(0.0);
                crate::models::DetectedTable {
                    bbox: (
                        (min_x - eff_box.x0) as f32,
                        (eff_box.y1 - max_y) as f32,
                        (max_x - min_x) as f32,
                        (max_y - min_y) as f32,
                    ),
                    csv: t.to_csv(),
                    tsv: t.to_tsv(),
                    cells: t.cells,
                }
            })
            .collect())
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
            for (idx, pixel) in data.as_chunks::<4>().0.iter().enumerate() {
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

    /// Optimize a PDF: stream compression plus image re-encoding.
    ///
    /// Two passes, because neither alone is sufficient:
    ///
    /// 1. `rewrite_pdf` garbage-collects unreferenced objects and deflates
    ///    unfiltered streams. Its `max_image_dimension` downsampler is narrow —
    ///    it only accepts 8-bit `DeviceRGB`/`DeviceGray` Flate images with no
    ///    `/Mask`, `/Decode` or `/DecodeParms` — so it skips the `DCTDecode`
    ///    and `CCITTFaxDecode`/`JBIG2Decode` streams that dominate scanned and
    ///    photo-heavy documents.
    /// 2. [`crate::image_optimizer`] re-encodes exactly those images, halving
    ///    their effective DPI (floored at 150 dpi) and re-emitting them as
    ///    baseline JPEG or JBIG2.
    ///
    /// The order matters: pass 1 renumbers every object, so the object ids the
    /// image pass works with are only valid against the *original* parse. The
    /// image pass therefore runs first, its replacements are applied to the
    /// original document via `IncrementalWriter`, and the rewritten result is
    /// then garbage-collected and compressed as usual.
    pub fn optimize_pdf(&self, input_path: &str, output_path: &str) -> PdfResult<String> {
        let data = read_input_capped(input_path)?;

        // `PdfFile::parse` takes ownership of the bytes, so the buffer it parses
        // *is* the document — there is no separate copy to free. The only place
        // a second full-size buffer was needed was the image analysis pass,
        // which must borrow the original while `apply_image_replacements`
        // consumes it. That pass is skipped outright for a document with no
        // image XObjects, and its result buffer is released as soon as the
        // replacements have been planned.
        let staged_pdf = if self.has_placed_images(&data) {
            let pdf = zpdf::PdfFile::parse(data.clone())
                .map_err(|e| PdfError::OpenFailed(e.to_string()))?;

            let dpi = crate::image_optimizer::collect_placed_dpi(&pdf);
            let (replacements, stats) = crate::image_optimizer::plan_replacements(&pdf, &dpi);
            tracing::info!(
                images_rewritten = stats.images_rewritten,
                images_skipped = stats.images_skipped,
                image_bytes_saved = stats.image_bytes_saved,
                "image re-encoding pass"
            );
            // Release the analysis parse (and its copy of the bytes) before
            // allocating the staged revision.
            drop(pdf);

            if replacements.is_empty() {
                // Nothing to stage: hand the original buffer straight to the
                // final parse rather than building a second full-size copy.
                zpdf::PdfFile::parse(data).map_err(|e| PdfError::OpenFailed(e.to_string()))?
            } else {
                let staged = self.apply_image_replacements(data, &replacements)?;
                zpdf::PdfFile::parse(staged).map_err(|e| PdfError::OpenFailed(e.to_string()))?
            }
        } else {
            zpdf::PdfFile::parse(data).map_err(|e| PdfError::OpenFailed(e.to_string()))?
        };

        let opts = RewriteOptions {
            compress_uncompressed: true,
            // The image pass above handles downsampling at a DPI the document
            // actually implies, which is strictly better than a fixed pixel cap
            // on images of unknown placement.
            max_image_dimension: None,
            ..Default::default()
        };
        let out_bytes =
            rewrite_pdf(&staged_pdf, &opts).map_err(|e| PdfError::IoError(e.to_string()))?;
        drop(staged_pdf);
        write_pdf_output(output_path, &out_bytes)?;
        Ok(output_path.to_string())
    }

    /// Whether `data` contains any placed image at all.
    ///
    /// Used purely to decide whether `optimize_pdf` needs to clone the input
    /// buffer for the image pass. A cheap early-out on the raw bytes: if the
    /// substring `b"/Image"` does not occur, no object can be an image XObject.
    fn has_placed_images(&self, data: &[u8]) -> bool {
        // Not a security boundary, just a cheap way to skip a full-size clone
        // for the image pass: if no object names an image subtype, none exists.
        //
        // The window width must equal the needle length. Comparing a 5-byte
        // window against `b"/Image"` (6 bytes) is *always* false, so the
        // previous form returned `false` for every input and `optimize_pdf`
        // skipped its image pass unconditionally.
        const IMAGE_SUBTYPES: [&[u8]; 4] =
            [b"/Image", b"/DCTDecode", b"/JPXDecode", b"/JBIG2Decode"];
        IMAGE_SUBTYPES
            .iter()
            .any(|needle| data.windows(needle.len()).any(|w| w == *needle))
    }

    /// Overwrite each replaced image stream in an incremental update.
    ///
    /// `IncrementalWriter` appends a new revision shadowing the original
    /// objects, so untouched parts of the document — including the original
    /// image bytes for every image left alone — are preserved verbatim. A
    /// failure part-way through writes nothing: the caller keeps the original
    /// bytes and the optimization degrades to a no-op.
    fn apply_image_replacements(
        &self,
        data: Vec<u8>,
        replacements: &[(zpdf::ObjectId, crate::image_optimizer::Replacement)],
    ) -> PdfResult<Vec<u8>> {
        let mut writer =
            IncrementalWriter::new(data).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        for (id, replacement) in replacements {
            let stream = zpdf::PdfObject::Stream(zpdf::PdfStream {
                dict: replacement.dict.clone(),
                data: replacement.data.clone().into(),
            });
            writer.overwrite_object(*id, stream);
        }
        let mut buf = std::io::Cursor::new(Vec::new());
        writer
            .write(&mut buf)
            .map_err(|e| PdfError::IoError(e.to_string()))?;
        Ok(buf.into_inner())
    }

    pub fn encrypt_pdf(
        &self,
        input_path: &str,
        output_path: &str,
        user_pass: &str,
        owner_pass: &str,
        algorithm: &str,
    ) -> PdfResult<String> {
        let data = read_input_capped(input_path)?;
        let pdf = zpdf::PdfFile::parse(data).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        let enc_config = if algorithm.eq_ignore_ascii_case("rc4") {
            zpdf_writer::EncryptionConfig::rc4_128(user_pass, owner_pass)
        } else {
            zpdf_writer::EncryptionConfig::aes256(user_pass, owner_pass)
        };
        let opts = RewriteOptions {
            encrypt: Some(enc_config),
            ..Default::default()
        };
        let out_bytes = rewrite_pdf(&pdf, &opts).map_err(|e| PdfError::IoError(e.to_string()))?;
        write_pdf_output(output_path, &out_bytes)?;
        Ok(output_path.to_string())
    }

    pub fn linearize_pdf(&self, input_path: &str, output_path: &str) -> PdfResult<String> {
        let data = read_input_capped(input_path)?;
        let pdf = zpdf::PdfFile::parse(data).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        let out_bytes =
            zpdf_writer::linearize_pdf(&pdf).map_err(|e| PdfError::IoError(e.to_string()))?;
        write_pdf_output(output_path, &out_bytes)?;
        Ok(output_path.to_string())
    }

    pub fn tag_pdf(&self, input_path: &str, output_path: &str) -> PdfResult<String> {
        let data = read_input_capped(input_path)?;
        let mut writer =
            IncrementalWriter::new(data).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        writer
            .tag_pdf()
            .map_err(|e| PdfError::IoError(e.to_string()))?;
        let out_bytes = {
            let mut buf = std::io::Cursor::new(Vec::new());
            writer
                .write(&mut buf)
                .map_err(|e| PdfError::IoError(e.to_string()))?;
            buf.into_inner()
        };
        write_pdf_output(output_path, &out_bytes)?;
        Ok(output_path.to_string())
    }

    pub fn validate_conformance(
        &self,
        path: &str,
        profile_str: &str,
    ) -> PdfResult<crate::models::ConformanceReport> {
        let data = read_input_capped(path)?;
        let pdf = zpdf::PdfFile::parse(data).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        let normalized = profile_str.to_lowercase().replace(['-', '/', '_', ' '], "");
        match normalized.as_str() {
            "pdfa1b" | "1b" | "a1b" => {
                let profile = zpdf::pdfa::Profile::A1b;
                let report = zpdf::pdfa::validate(&pdf, profile);
                let conforms = report.conforms();
                let claimed = report
                    .claimed
                    .as_ref()
                    .map(|(part, conf)| vec![("pdfaid".to_string(), format!("{part}/{conf}"))])
                    .unwrap_or_default();
                Ok(crate::models::ConformanceReport {
                    family: crate::models::ConformanceFamily::PdfA,
                    profile: "PDF/A-1b".to_string(),
                    conforms,
                    claimed,
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
            "pdfa2b" | "2b" | "a2b" => {
                let profile = zpdf::pdfa::Profile::A2b;
                let report = zpdf::pdfa::validate(&pdf, profile);
                let conforms = report.conforms();
                let claimed = report
                    .claimed
                    .as_ref()
                    .map(|(part, conf)| vec![("pdfaid".to_string(), format!("{part}/{conf}"))])
                    .unwrap_or_default();
                Ok(crate::models::ConformanceReport {
                    family: crate::models::ConformanceFamily::PdfA,
                    profile: "PDF/A-2b".to_string(),
                    conforms,
                    claimed,
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
            "pdfa3b" | "3b" | "a3b" => {
                let profile = zpdf::pdfa::Profile::A3b;
                let report = zpdf::pdfa::validate(&pdf, profile);
                let conforms = report.conforms();
                let claimed = report
                    .claimed
                    .as_ref()
                    .map(|(part, conf)| vec![("pdfaid".to_string(), format!("{part}/{conf}"))])
                    .unwrap_or_default();
                Ok(crate::models::ConformanceReport {
                    family: crate::models::ConformanceFamily::PdfA,
                    profile: "PDF/A-3b".to_string(),
                    conforms,
                    claimed,
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
            "pdfx1a" | "1a" | "x1a" => {
                let profile = zpdf::pdfx::Profile::X1a;
                let report = zpdf::pdfx::validate(&pdf, profile);
                let conforms = report.conforms();
                let claimed = report
                    .claimed
                    .as_ref()
                    .map(|val| vec![("GTS_PDFXVersion".to_string(), val.clone())])
                    .unwrap_or_default();
                Ok(crate::models::ConformanceReport {
                    family: crate::models::ConformanceFamily::PdfX,
                    profile: "PDF/X-1a".to_string(),
                    conforms,
                    claimed,
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
            "pdfx3" | "x3" => {
                let profile = zpdf::pdfx::Profile::X3;
                let report = zpdf::pdfx::validate(&pdf, profile);
                let conforms = report.conforms();
                let claimed = report
                    .claimed
                    .as_ref()
                    .map(|val| vec![("GTS_PDFXVersion".to_string(), val.clone())])
                    .unwrap_or_default();
                Ok(crate::models::ConformanceReport {
                    family: crate::models::ConformanceFamily::PdfX,
                    profile: "PDF/X-3".to_string(),
                    conforms,
                    claimed,
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
            "pdfx4" | "x4" => {
                let profile = zpdf::pdfx::Profile::X4;
                let report = zpdf::pdfx::validate(&pdf, profile);
                let conforms = report.conforms();
                let claimed = report
                    .claimed
                    .as_ref()
                    .map(|val| vec![("GTS_PDFXVersion".to_string(), val.clone())])
                    .unwrap_or_default();
                Ok(crate::models::ConformanceReport {
                    family: crate::models::ConformanceFamily::PdfX,
                    profile: "PDF/X-4".to_string(),
                    conforms,
                    claimed,
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
            "pdfx6" | "x6" => {
                let profile = zpdf::pdfx::Profile::X6;
                let report = zpdf::pdfx::validate(&pdf, profile);
                let conforms = report.conforms();
                let claimed = report
                    .claimed
                    .as_ref()
                    .map(|val| vec![("GTS_PDFXVersion".to_string(), val.clone())])
                    .unwrap_or_default();
                Ok(crate::models::ConformanceReport {
                    family: crate::models::ConformanceFamily::PdfX,
                    profile: "PDF/X-6".to_string(),
                    conforms,
                    claimed,
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
            "pdfua1" | "ua1" => {
                let profile = zpdf::pdfua::Profile::Ua1;
                let report = zpdf::pdfua::validate(&pdf, profile);
                let conforms = report.conforms();
                Ok(crate::models::ConformanceReport {
                    family: crate::models::ConformanceFamily::PdfUa,
                    profile: "PDF/UA-1".to_string(),
                    conforms,
                    claimed: Vec::new(),
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
            "pdfua2" | "ua2" => {
                let profile = zpdf::pdfua::Profile::Ua2;
                let report = zpdf::pdfua::validate(&pdf, profile);
                let conforms = report.conforms();
                Ok(crate::models::ConformanceReport {
                    family: crate::models::ConformanceFamily::PdfUa,
                    profile: "PDF/UA-2".to_string(),
                    conforms,
                    claimed: Vec::new(),
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
            _ => Err(PdfError::OpenFailed(format!(
                "Unsupported conformance profile: '{profile_str}'"
            ))),
        }
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
        let custom_anchors;
        let anchors: &[zpdf::trust::TrustAnchor] = if trust_anchors_bytes.is_empty() {
            Self::cached_system_trust_anchors()
        } else {
            custom_anchors = zpdf::trust::parse_trust_anchors(trust_anchors_bytes);
            &custom_anchors
        };
        let sigs = doc.signatures();
        let mut results = Vec::new();
        for sig in sigs {
            let status = if let Some(ref cms) = sig.cms_blob {
                let chain_status = zpdf::trust::verify_certificate_chain(cms, anchors, None);
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

    fn convert_inner(doc: &PdfDocument, mode: &str, format: &str) -> PdfResult<String> {
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
        Self::convert_inner(doc, mode, format)
    }

    pub fn convert_pdf_doc(&self, path: &str, mode: &str, format: &str) -> PdfResult<String> {
        let metadata = std::fs::metadata(path).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        if metadata.len() > 512 * 1024 * 1024 {
            return Err(PdfError::OpenFailed(
                "PDF file exceeds 512 MB size limit".into(),
            ));
        }
        let data = read_input_capped(path)?;
        let doc = zpdf::PdfDocument::open(data).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        Self::convert_inner(&doc, mode, format)
    }

    /// Merge `documents` (path plus optional open password, in order) into
    /// `output_path`.
    ///
    /// Each document carries its own password: an encrypted input used to be
    /// unreadable here because the writer was always built with the empty
    /// password, so "Merge" failed with an opaque parse error on any protected
    /// file.
    pub fn merge_documents(
        &self,
        documents: Vec<(String, Option<String>)>,
        output_path: String,
    ) -> PdfResult<String> {
        let Some((first_path, first_password)) = documents.first() else {
            return Err(PdfError::IoError("No documents to merge".into()));
        };

        // Load and use the first document as the base of the IncrementalWriter.
        // Capped like the documents appended below: a bare `std::fs::read` here
        // allocated room for the whole file before any size check, so a
        // multi-GB input OOMs the process instead of being refused.
        let first_data = read_input_capped(first_path)?;
        let mut writer = IncrementalWriter::new_with_password(
            first_data,
            first_password.as_deref().unwrap_or("").as_bytes(),
        )
        .map_err(|e| PdfError::OpenFailed(e.to_string()))?;

        // Append remaining documents — this preserves outlines, forms, and OCGs.
        for (path, password) in documents.iter().skip(1) {
            let data = read_input_capped(path)?;
            let doc = zpdf::PdfFile::parse_with_password(
                data,
                password.as_deref().unwrap_or("").as_bytes(),
            )
            .map_err(|e| PdfError::OpenFailed(e.to_string()))?;
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
        write_pdf_output(&output_path, &out_bytes)?;

        Ok(output_path)
    }

    pub fn reorder_pages(
        &self,
        input_path: &str,
        page_order: &[usize],
        output_path: &str,
        password: Option<&str>,
    ) -> PdfResult<String> {
        let data = read_input_capped(input_path)?;
        let mut writer =
            IncrementalWriter::new_with_password(data, password.unwrap_or("").as_bytes())
                .map_err(|e| PdfError::OpenFailed(e.to_string()))?;

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
        write_pdf_output(output_path, &out_bytes)?;

        Ok(output_path.to_string())
    }

    pub fn rotate_pages(
        &self,
        input_path: &str,
        rotations: &[(usize, i32)],
        output_path: &str,
        password: Option<&str>,
    ) -> PdfResult<String> {
        let data = read_input_capped(input_path)?;
        let mut writer =
            IncrementalWriter::new_with_password(data, password.unwrap_or("").as_bytes())
                .map_err(|e| PdfError::OpenFailed(e.to_string()))?;

        for &(page_idx, degrees) in rotations {
            writer
                .rotate_page(page_idx, degrees)
                .map_err(|e| PdfError::IoError(e.to_string()))?;
        }

        let out_bytes = {
            let mut buf = std::io::Cursor::new(Vec::new());
            writer
                .write(&mut buf)
                .map_err(|e| PdfError::IoError(e.to_string()))?;
            buf.into_inner()
        };
        write_pdf_output(output_path, &out_bytes)?;

        Ok(output_path.to_string())
    }

    pub fn delete_pages(
        &self,
        input_path: &str,
        page_indices: &[usize],
        output_path: &str,
        password: Option<&str>,
    ) -> PdfResult<String> {
        let data = read_input_capped(input_path)?;
        let mut writer =
            IncrementalWriter::new_with_password(data, password.unwrap_or("").as_bytes())
                .map_err(|e| PdfError::OpenFailed(e.to_string()))?;

        writer
            .delete_pages(page_indices)
            .map_err(|e| PdfError::IoError(e.to_string()))?;

        let out_bytes = {
            let mut buf = std::io::Cursor::new(Vec::new());
            writer
                .write(&mut buf)
                .map_err(|e| PdfError::IoError(e.to_string()))?;
            buf.into_inner()
        };
        write_pdf_output(output_path, &out_bytes)?;

        Ok(output_path.to_string())
    }

    pub fn export_page_svg(&self, doc_id: DocumentId, page_num: usize) -> PdfResult<String> {
        let doc = self
            .documents
            .get(&doc_id)
            .ok_or(PdfError::EngineError(EngineErrorKind::DocumentNotFound))?;
        let interpreted =
            zpdf::interpret_page(doc, page_num, zpdf::PageInterpretOptions::default())
                .map_err(|e| PdfError::RenderFailed(e.to_string()))?;

        let svg = zpdf_svg_export::display_list_to_svg(
            &interpreted.display_list,
            &interpreted.fonts,
            &interpreted.images,
            &zpdf_svg_export::SvgOptions::default(),
        );
        Ok(svg)
    }

    pub fn split_pdf(
        &self,
        path: &str,
        page_indices: Vec<usize>,
        output_dir: String,
        password: Option<&str>,
    ) -> PdfResult<Vec<String>> {
        let base_data = read_input_capped(path)?;

        // The authoritative page count is the length of the *page tree*, not the
        // number of objects carrying `/Type /Page`. Those differ whenever a
        // document has accumulated orphaned page objects (left behind by
        // incremental updates) or omits `/Type` on a leaf and inherits it from
        // `/Pages`.
        //
        // An inflated count let the UI's own `(0..total_pages)` range send an
        // out-of-tree index that passed the bounds check and reached
        // `extract_pages`. A deflated count silently skipped valid indices with
        // only a `warn!`, so pages the user asked for were never produced.
        //
        // `PdfDocument` resolves the tree exactly as `open_document` does and
        // exposes the same underlying `PdfFile` via `file()`, so this does not
        // re-read or re-serialize the document.
        //
        // This path historically took no password, so an **encrypted** document
        // could not be split at all. A password can now be supplied; without one
        // the open reports `PasswordRequired` rather than an opaque parse error,
        // so the caller can ask for one. (There is no object-scan fallback: the
        // page tree cannot be walked while the file is encrypted.)
        let doc =
            zpdf::PdfDocument::open_with_password(base_data, password.unwrap_or("").as_bytes())
                .map_err(|e| match e {
                    zpdf::Error::WrongPassword | zpdf::Error::PasswordRequired => {
                        PdfError::PasswordRequired
                    }
                    other => PdfError::OpenFailed(other.to_string()),
                })?;
        let pdf_file = doc.file();
        let total_pages = doc.page_count();
        if total_pages == 0 {
            return Err(PdfError::OpenFailed(
                "document reports no pages".to_string(),
            ));
        }

        let filename = std::path::Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("document");

        // An empty selection is a caller mistake, not an empty success. The UI
        // would otherwise report "split into 0 pages" for a request that was
        // never going to produce anything.
        if page_indices.is_empty() {
            return Err(PdfError::OpenFailed(
                "no pages were selected to split".to_string(),
            ));
        }

        let mut created_paths = Vec::new();

        // Ensure the destination directory exists rather than failing on the
        // first write.
        let _ = std::fs::create_dir_all(&output_dir);

        // Out-of-range indices are collected rather than silently dropped: a
        // caller that cannot tell "split" from "split nothing" reports success
        // for a document it never touched.
        let mut skipped: Vec<String> = Vec::new();

        for &page_idx in &page_indices {
            if page_idx >= total_pages {
                tracing::warn!(
                    "split_pdf: skipping page {} (document has {total_pages} pages)",
                    page_idx + 1
                );
                skipped.push(format!("page {} (max {total_pages})", page_idx + 1));
                continue;
            }

            let out_bytes = zpdf::extract_pages(pdf_file, &[page_idx])
                .map_err(|e| PdfError::IoError(e.to_string()))?;

            let out_path = std::path::Path::new(&output_dir)
                .join(format!("{filename}_page_{}.pdf", page_idx + 1))
                .to_string_lossy()
                .into_owned();
            write_pdf_output(&out_path, &out_bytes)?;
            created_paths.push(out_path);
        }

        if created_paths.is_empty() {
            return Err(PdfError::IoError(format!(
                "no pages could be extracted from {path}: {}",
                if skipped.is_empty() {
                    "none were selected".to_string()
                } else {
                    skipped.join("; ")
                }
            )));
        }

        Ok(created_paths)
    }

    /// Burn redaction annotations permanently into the PDF, making the
    /// redacted content irrecoverable. Goes through zpdf's destructive
    /// `redact_pdf` (0.17+): after removing the matching text/XObject/path
    /// operators from the page content, the whole document is rewritten so
    /// the superseded revisions — in particular the **original, unredacted
    /// content stream** — are absent from the bytes we write.
    /// `IncrementalWriter::redact_page` alone is not sufficient: it appends a
    /// new revision and leaves the original file prefix in the output, which
    /// is exactly what a redaction must not do.
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

        let data = read_input_capped(pdf_path)?;

        // `redact_pdf` redacts one page per call and rebuilds the whole file,
        // so feed each call the previous call's output. Pages are visited in
        // ascending order (BTreeMap) so the result does not depend on the
        // annotation collection's map iteration order.
        let mut by_page: std::collections::BTreeMap<usize, Vec<ZpdfRect>> = Default::default();
        for ann in annotations {
            if let AnnotationStyle::Redact { .. } = &ann.style {
                // Convert from screen (top-left, box-relative) coords back to PDF
                // (bottom-left, box-absolute) coords, mirroring
                // `save_annotations`. Using the box *height* here and dropping
                // its own `x0`/`y1` burned the wrong region on every page whose
                // box does not start at the origin: a `CropBox` with a non-zero
                // left edge redacted an area shifted by exactly that edge.
                // A guessed box would burn the wrong region — refuse.
                let page_box = self
                    .documents
                    .get(&doc_id)
                    .and_then(|d| d.page(ann.page).ok())
                    .map(|p| p.effective_box())
                    .ok_or_else(|| {
                        PdfError::EngineError(EngineErrorKind::Generic(format!(
                            "cannot redact: page {} of the document is not loaded",
                            ann.page + 1
                        )))
                    })?;

                let pdf_x = ann.x as f64 + page_box.x0;
                let pdf_y = page_box.y1 - ann.y as f64 - ann.height as f64;
                let rect = ZpdfRect {
                    x0: pdf_x,
                    y0: pdf_y,
                    x1: pdf_x + ann.width as f64,
                    y1: pdf_y + ann.height as f64,
                };
                by_page.entry(ann.page).or_default().push(rect);
            }
        }

        // Empty password matches what `IncrementalWriter::new` accepted for
        // this code's callers: documents redacted here were opened by the
        // engine with a password the caller already supplied, but the writer
        // path never carried it. `redact_pdf` maps `WrongPassword` onto the
        // same open error as before — nothing silently succeeds on an
        // encrypted file.
        let pass = self
            .passwords
            .get(&doc_id)
            .map(|s| s.as_bytes())
            .unwrap_or(b"");
        let mut out_bytes = data;
        for (page_idx, rects) in by_page {
            out_bytes = redact_pdf(out_bytes, pass, page_idx, &rects, &RedactOptions::default())
                .map_err(|e| PdfError::IoError(e.to_string()))?;
        }
        // `redact_pdf` decrypts while rewriting (its output cannot retain the
        // encryption dictionary, only the password it was given — which here
        // is empty), so the written file is unencrypted even if the source
        // was not. Redacted output leaving the app unencrypted is the same
        // posture as `save_annotations`'s write path.

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
            if dir.is_empty() {
                format!("{}_redacted.pdf", stem)
            } else {
                format!("{}/{}_redacted.pdf", dir, stem)
            }
        });

        write_pdf_output(&final_path, &out_bytes)?;

        Ok(final_path)
    }

    /// Map every widget annotation object id to the page index it appears on.
    ///
    /// One pass over the page tree, so `extract_form_fields_from_doc` can look
    /// up a field's page in constant time instead of re-scanning every page for
    /// every field.
    fn index_widget_pages(doc: &PdfDocument) -> std::collections::HashMap<zpdf::ObjectId, usize> {
        let mut map = std::collections::HashMap::new();
        for i in 0..doc.page_count() {
            let Ok(page) = doc.page(i) else {
                continue;
            };
            for widget in &page.annots {
                // First page wins, matching the old "break at the first match"
                // behaviour for a widget placed on more than one page.
                map.entry(*widget).or_insert(i);
            }
        }
        map
    }

    pub fn get_form_fields(&mut self, path: &str) -> PdfResult<Vec<FormField>> {
        let doc_id = self
            .paths
            .iter()
            .find(|(_, p)| *p == path)
            .map(|(id, _)| *id);

        if let Some(id) = doc_id
            && let Some(doc) = self.documents.get(&id)
        {
            return Ok(self.extract_form_fields_from_doc(doc));
        }

        let data = read_input_capped(path)?;
        let doc = PdfDocument::open(data).map_err(|e| PdfError::OpenFailed(e.to_string()))?;
        Ok(self.extract_form_fields_from_doc(&doc))
    }

    fn extract_form_fields_from_doc(&self, doc: &PdfDocument) -> Vec<FormField> {
        let mut fields = Vec::new();
        if let Some(acro) = doc.acro_form() {
            // Build a widget-id -> page-index map in a single pass over the
            // document.
            //
            // The previous code called `doc.page(i)` for every field, re-parsing
            // every page from 0 until it found the field's widget. That is
            // O(fields x pages) full page parses: a 200-field form on a
            // 5,000-page document meant up to a million page parses on the
            // engine thread, stalling every other command in the queue.
            //
            // It also misreported the page: a widget that appears on no page --
            // common when a field's widget lives in an inheritance chain, or is
            // a parent field with `/Kids` -- left `page_idx` at its `0`
            // initializer, so the field was reported on page 1 with no warning
            // and the UI scrolled somewhere the user could not see the field.
            // Such fields now carry `None`.
            let widget_pages = Self::index_widget_pages(doc);

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
                        let (is_checked, on_value) = match &f.value {
                            Some(FieldValue::Name(n)) if n != "Off" => (true, Some(n.clone())),
                            // `/V /Off` is how an *unchecked* button records its
                            // state, so it says nothing about the on-value.
                            // Carrying it as `Some("Off")` made a fill write
                            // "Off" straight back, leaving the box unchecked
                            // after the user had asked to check it.
                            _ => (false, None),
                        };
                        if f.flags & FF_RADIO != 0 {
                            FormFieldVariant::RadioButton {
                                is_selected: is_checked,
                                group_name: Some(name.clone()),
                                on_value,
                            }
                        } else {
                            FormFieldVariant::Checkbox {
                                is_checked,
                                on_value,
                            }
                        }
                    }
                    FieldKind::Choice => {
                        // `/Opt` entries pair an export value with a label.
                        // Keep both: the label is what a person picks, the
                        // export value is what belongs in `/V`.
                        let opts: Vec<String> =
                            f.options.iter().map(|(_, label)| label.clone()).collect();
                        let exports: Vec<String> = f
                            .options
                            .iter()
                            .map(|(export, label)| {
                                if export.is_empty() {
                                    label.clone()
                                } else {
                                    export.clone()
                                }
                            })
                            .collect();
                        let selected_val = match &f.value {
                            Some(FieldValue::Text(s)) => Some(s.clone()),
                            _ => None,
                        };
                        // Match on the export value first, then fall back to the
                        // label, since some producers write the label into `/V`.
                        let selected_index = selected_val.and_then(|val| {
                            f.options
                                .iter()
                                .position(|(export, _)| *export == val)
                                .or_else(|| f.options.iter().position(|(_, label)| *label == val))
                        });
                        FormFieldVariant::ComboBox {
                            options: opts,
                            export_values: exports,
                            selected_index,
                        }
                    }
                    _ => FormFieldVariant::Text {
                        value: String::new(),
                    },
                };

                // Resolve the field's page from the prebuilt index. A widget
                // that appears on no page falls back to 0, which is logged so
                // the misreport is at least visible rather than silent.
                let page = f.widgets.iter().find_map(|w| widget_pages.get(w).copied());
                if page.is_none() {
                    tracing::debug!(
                        "form field '{name}' has a widget that appears on no page; \
                         reporting page 0"
                    );
                }

                fields.push(FormField {
                    name,
                    variant,
                    page: page.unwrap_or(0),
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
        password: Option<&str>,
    ) -> PdfResult<String> {
        let data = read_input_capped(path).map_err(|e| match e {
            PdfError::OpenFailed(msg) => PdfError::IoError(msg),
            other => other,
        })?;
        let mut writer =
            IncrementalWriter::new_with_password(data, password.unwrap_or("").as_bytes())
                .map_err(|e| PdfError::IoError(e.to_string()))?;

        let total_fields = updates.len();
        let mut failures: Vec<String> = Vec::new();

        {
            let mut filler =
                FormFiller::new(&mut writer).map_err(|e| PdfError::IoError(e.to_string()))?;
            for update in updates {
                let val_str = match &update.variant {
                    FormFieldVariant::Text { value } => value.clone(),
                    FormFieldVariant::Checkbox {
                        is_checked,
                        on_value,
                    } => {
                        if *is_checked {
                            on_value.clone().unwrap_or_else(|| "Yes".to_string())
                        } else {
                            "Off".to_string()
                        }
                    }
                    FormFieldVariant::RadioButton {
                        is_selected,
                        on_value,
                        ..
                    } => {
                        if *is_selected {
                            on_value.clone().unwrap_or_else(|| "Yes".to_string())
                        } else {
                            "Off".to_string()
                        }
                    }
                    FormFieldVariant::ComboBox {
                        selected_index,
                        options,
                        export_values,
                    } => {
                        if let Some(idx) = selected_index {
                            // Write the *export value*, not the label. The old
                            // code wrote `options[idx]`, which is the display
                            // text, so any choice field whose value differed from
                            // its label was saved with a `/V` no viewer matches.
                            export_values
                                .get(*idx)
                                .cloned()
                                .or_else(|| options.get(*idx).cloned())
                                .unwrap_or_default()
                        } else {
                            // No selection changed; do not overwrite or clear the existing value.
                            continue;
                        }
                    }
                };
                if let Err(e) = filler.set(&update.name, &val_str) {
                    // A rejected field means the user's input was silently
                    // dropped. Reporting success here produced a PDF with
                    // nothing filled in and no indication anything went wrong,
                    // so collect the failures and surface them.
                    failures.push(format!("{}: {e}", update.name));
                }
            }
            filler
                .finish()
                .map_err(|e| PdfError::IoError(e.to_string()))?;
        }

        // Render into memory first. `File::create` truncates the destination
        // before a single new byte exists, so writing a form back over the path
        // it was opened from (which the save dialog allows) destroyed the
        // original if anything below failed.
        let mut buf = std::io::Cursor::new(Vec::new());
        writer
            .write(&mut buf)
            .map_err(|e| PdfError::IoError(e.to_string()))?;

        // Refuse *before* writing. The write below replaces the destination
        // atomically, and the save dialog permits that destination to be the file
        // the form was opened from — so writing a partially-filled form and then
        // reporting failure replaced the user's original with an artifact this
        // function had just declared invalid.
        if !failures.is_empty() {
            return Err(PdfError::EngineError(EngineErrorKind::Generic(format!(
                "filled {} of {} fields; these were rejected: {}",
                total_fields.saturating_sub(failures.len()),
                total_fields,
                failures.join("; ")
            ))));
        }

        write_pdf_output(&output_path, &buf.into_inner())?;

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
        // Capped, like every other input path: a bare `Document::load` reads the
        // whole file with no size check, so a multi-GB input OOMs the process
        // rather than being rejected up front.
        let input = read_input_capped(input_path)?;
        let mut doc =
            Document::load_mem(&input).map_err(|e| PdfError::OpenFailed(e.to_string()))?;

        let pages: Vec<ObjectId> = doc.get_pages().into_values().collect();
        let page_count = pages.len();

        let font_ref_id = doc.add_object(Object::Dictionary(lopdf::Dictionary::from_iter(vec![
            ("Type", Object::Name(b"Font".to_vec())),
            ("Subtype", Object::Name(b"Type1".to_vec())),
            ("BaseFont", Object::Name(b"Helvetica".to_vec())),
        ])));

        let mut gstate = lopdf::Dictionary::new();
        gstate.set("Type", Object::Name(b"ExtGState".to_vec()));
        gstate.set("ca", Object::Real(0.35));
        gstate.set("CA", Object::Real(0.35));
        let gstate_id = doc.add_object(Object::Dictionary(gstate));

        let resources_id = doc.add_object(Object::Dictionary(lopdf::Dictionary::from_iter(vec![
            (
                "Font",
                Object::Dictionary(lopdf::Dictionary::from_iter(vec![(
                    "F1",
                    Object::Reference(font_ref_id),
                )])),
            ),
            (
                "ExtGState",
                Object::Dictionary(lopdf::Dictionary::from_iter(vec![(
                    "GS_WM",
                    Object::Reference(gstate_id),
                )])),
            ),
        ])));

        // Encode once, outside the per-page loop: the bytes go into every page's
        // content stream, so re-encoding per page only repeated the work.
        let (text_bytes, unmappable) = encode_pdf_text(text);
        if unmappable > 0 {
            tracing::warn!(
                "Watermark text has {unmappable} character(s) outside WinAnsiEncoding \
                 (e.g. CJK or emoji); substituted '?' because the non-embedded \
                 Helvetica font cannot draw them"
            );
        }

        // The watermark content stream is built *per page* so its placement can
        // follow that page's box. It used to be built once with a hard-coded
        // origin of (200, 400), which put the stamp off the page — or absurdly
        // off-centre — for any page whose MediaBox is not A4-sized.
        for &page_id in &pages {
            let (x0, y0, x1, y1) =
                effective_page_box(&doc, page_id).unwrap_or((0.0, 0.0, 595.0, 842.0));
            let (page_w, page_h) = (x1 - x0, y1 - y0);
            let rot = page_rotation_lopdf(&doc, page_id);

            let mut content = pdf_writer::Content::new();
            content.save_state();
            content.set_parameters(pdf_writer::Name(b"GS_WM"));
            content.begin_text();
            // Helvetica averages ~0.6 em per character; cap the size so a long
            // stamp still fits the page width, then centre the run at
            // mid-height so it reads as a watermark on any page size.
            let font_size = 48.0_f32.min((page_w / 12.0).max(12.0));
            content.set_font(pdf_writer::Name(b"F1"), font_size);
            content.set_fill_rgb(0.5, 0.5, 0.5);
            let estimated_w = (text_bytes.len() as f32) * font_size * 0.6;
            match rot {
                90 => {
                    let x = (x0 + page_w / 2.0).max(x0 + 10.0);
                    let y = (y0 + (page_h - estimated_w) / 2.0).max(y0 + 10.0);
                    content.set_text_matrix([0.0, 1.0, -1.0, 0.0, x, y]);
                }
                180 => {
                    let x = (x0 + (page_w + estimated_w) / 2.0).min(x1 - 10.0);
                    let y = (y0 + page_h / 2.0).max(y0 + 10.0);
                    content.set_text_matrix([-1.0, 0.0, 0.0, -1.0, x, y]);
                }
                270 => {
                    let x = (x0 + page_w / 2.0).max(x0 + 10.0);
                    let y = (y0 + (page_h + estimated_w) / 2.0).min(y1 - 10.0);
                    content.set_text_matrix([0.0, -1.0, 1.0, 0.0, x, y]);
                }
                _ => {
                    let x = (x0 + (page_w - estimated_w) / 2.0).max(x0 + 10.0);
                    let y = (y0 + page_h / 2.0).max(y0 + 10.0);
                    content.set_text_matrix([1.0, 0.0, 0.0, 1.0, x, y]);
                }
            }
            content.show(pdf_writer::Str(&text_bytes));
            content.end_text();
            content.restore_state();
            let watermark_stream =
                lopdf::Stream::new(lopdf::Dictionary::new(), content.finish().to_vec());
            let watermark_id = doc.add_object(watermark_stream);

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
            if let Some(ref res) = existing_res {
                let mut res_dict = match res {
                    Object::Reference(r) => {
                        doc.objects.get(r).and_then(|o| o.as_dict().ok()).cloned()
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

                    if let Ok(existing_extgstate) = d.get_mut(b"ExtGState") {
                        let mut gstate_dict = match existing_extgstate {
                            Object::Reference(r) => {
                                doc.objects.get(r).and_then(|o| o.as_dict().ok()).cloned()
                            }
                            Object::Dictionary(gd) => Some(gd.clone()),
                            _ => None,
                        };
                        if let Some(ref mut gd) = gstate_dict {
                            gd.set("GS_WM", Object::Reference(gstate_id));
                            d.set("ExtGState", Object::Dictionary(gd.clone()));
                        }
                    } else {
                        d.set(
                            "ExtGState",
                            Object::Dictionary(lopdf::Dictionary::from_iter(vec![(
                                "GS_WM",
                                Object::Reference(gstate_id),
                            )])),
                        );
                    }

                    merged_res = Some(doc.add_object(Object::Dictionary(d.clone())));
                }
            }

            let page_dict = doc
                .objects
                .get_mut(&page_id)
                .and_then(|o| o.as_dict_mut().ok())
                .ok_or_else(|| PdfError::EngineError("Invalid page object".into()))?;
            page_dict.set("Contents", Object::Array(all_contents));
            // Only fall back to the shared font-only dictionary when the page
            // had no `/Resources` at all. `merged_res` stays `None` if the page
            // *did* have one that could not be resolved to a dictionary, and
            // assigning `resources_id` in that case replaced the page's real
            // resources — the same content-destroying outcome as the
            // header/footer path. Leave those pages untouched instead.
            match merged_res {
                Some(res_id) => page_dict.set("Resources", Object::Reference(res_id)),
                None if existing_res.is_none() => {
                    page_dict.set("Resources", Object::Reference(resources_id))
                }
                None => tracing::warn!(
                    "Watermark: page {page_id:?} has unresolvable /Resources; leaving it unchanged"
                ),
            }
        }

        // Serialize to memory, then write atomically. `doc.save` streams
        // straight into the destination, truncating it first.
        let mut bytes = Vec::new();
        doc.save_to(&mut bytes)
            .map_err(|e| PdfError::IoError(e.to_string()))?;
        write_pdf_output(output_path, &bytes)?;

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
        // Capped, like every other input path: a bare `Document::load` reads the
        // whole file with no size check, so a multi-GB input OOMs the process
        // rather than being rejected up front.
        let input = read_input_capped(input_path)?;
        let mut doc =
            Document::load_mem(&input).map_err(|e| PdfError::OpenFailed(e.to_string()))?;

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
            // Anchor the header and footer to this page's own box instead of a
            // hard-coded 760/30. On any page shorter than 760 pt tall the header
            // landed outside the MediaBox and silently vanished.
            let (x0, y0, x1, y1) =
                effective_page_box(&doc, page_id).unwrap_or((0.0, 0.0, 595.0, 842.0));
            let (page_w, page_h) = (x1 - x0, y1 - y0);
            let rot = page_rotation_lopdf(&doc, page_id);
            let margin = 36.0_f32.min(page_w / 8.0);

            let mut content = pdf_writer::Content::new();
            content.save_state();
            content.begin_text();
            content.set_font(pdf_writer::Name(b"F1"), 10.0);
            content.set_fill_rgb(0.2, 0.2, 0.2);

            if !header_text.is_empty() {
                match rot {
                    90 => {
                        content.set_text_matrix([
                            0.0,
                            1.0,
                            -1.0,
                            0.0,
                            (x1 - margin - 10.0).max(x0 + 10.0),
                            y0 + margin,
                        ]);
                    }
                    180 => {
                        content.set_text_matrix([
                            -1.0,
                            0.0,
                            0.0,
                            -1.0,
                            x1 - margin,
                            (y0 + margin + 10.0).min(y1 - 10.0),
                        ]);
                    }
                    270 => {
                        content.set_text_matrix([
                            0.0,
                            -1.0,
                            1.0,
                            0.0,
                            x0 + margin,
                            (y1 - margin - 10.0).max(y0 + 10.0),
                        ]);
                    }
                    _ => {
                        content.set_text_matrix([
                            1.0,
                            0.0,
                            0.0,
                            1.0,
                            x0 + margin,
                            (y0 + page_h - margin - 10.0).max(y0 + 10.0),
                        ]);
                    }
                }
                let (bytes, unmappable) = encode_pdf_text(header_text);
                if unmappable > 0 {
                    tracing::warn!(
                        "Header has {unmappable} character(s) outside WinAnsiEncoding; \
                         substituted '?'"
                    );
                }
                content.show(pdf_writer::Str(&bytes));
            }

            if !formatted_footer.is_empty() {
                match rot {
                    90 => {
                        content.set_text_matrix([
                            0.0,
                            1.0,
                            -1.0,
                            0.0,
                            (x0 + margin).min(x1 - 10.0),
                            y0 + margin,
                        ]);
                    }
                    180 => {
                        content.set_text_matrix([
                            -1.0,
                            0.0,
                            0.0,
                            -1.0,
                            x1 - margin,
                            (y1 - margin - 10.0).max(y0 + 10.0),
                        ]);
                    }
                    270 => {
                        content.set_text_matrix([
                            0.0,
                            -1.0,
                            1.0,
                            0.0,
                            (x1 - margin).max(x0 + 10.0),
                            (y1 - margin - 10.0).max(y0 + 10.0),
                        ]);
                    }
                    _ => {
                        content.set_text_matrix([
                            1.0,
                            0.0,
                            0.0,
                            1.0,
                            x0 + margin,
                            (y0 + margin).max(y0 + 10.0),
                        ]);
                    }
                }
                let (bytes, unmappable) = encode_pdf_text(&formatted_footer);
                if unmappable > 0 {
                    tracing::warn!(
                        "Footer has {unmappable} character(s) outside WinAnsiEncoding; \
                         substituted '?'"
                    );
                }
                content.show(pdf_writer::Str(&bytes));
            }

            content.end_text();
            content.restore_state();

            let hf_stream = lopdf::Stream::new(lopdf::Dictionary::new(), content.finish().to_vec());
            let hf_id = doc.add_object(hf_stream);

            let existing_contents = doc
                .get_page_contents(page_id)
                .into_iter()
                .map(Object::Reference)
                .collect::<Vec<_>>();
            let mut all_contents = existing_contents;
            all_contents.push(Object::Reference(hf_id));

            // Register `F1` in the page's *existing* resource dictionary.
            //
            // This used to assign a brand-new `/Resources` containing only
            // `{Font: {F1}}`, discarding whatever the page already had. Every
            // `/XObject`, `/ExtGState`, `/ColorSpace`, `/Shading` and `/Pattern`
            // entry was dropped, so all images, soft masks and shadings on every
            // page became unresolvable and the saved file rendered blank apart
            // from the header and footer — while the UI reported success. Merge
            // instead, exactly as `add_watermark` does.
            //
            // Computed before the mutable borrow of `doc.objects` below.
            let existing_res = doc
                .objects
                .get(&page_id)
                .and_then(|o| o.as_dict().ok())
                .and_then(|d| d.get(b"Resources").ok())
                .cloned();

            let merged_res = match existing_res {
                Some(res) => {
                    let res_dict = match res {
                        Object::Reference(r) => {
                            doc.objects.get(&r).and_then(|o| o.as_dict().ok()).cloned()
                        }
                        Object::Dictionary(d) => Some(d),
                        _ => None,
                    };
                    // A page whose `/Resources` is present but unresolvable is
                    // left alone rather than being handed the font-only
                    // dictionary, which would destroy its real resources.
                    res_dict.map(|mut d| {
                        let fonts = match d.get(b"Font") {
                            Ok(Object::Reference(r)) => {
                                doc.objects.get(r).and_then(|o| o.as_dict().ok()).cloned()
                            }
                            Ok(Object::Dictionary(fd)) => Some(fd.clone()),
                            _ => None,
                        };
                        let mut fd = fonts.unwrap_or_default();
                        fd.set("F1", Object::Reference(font_ref_id));
                        d.set("Font", Object::Dictionary(fd));
                        doc.add_object(Object::Dictionary(d))
                    })
                }
                // No `/Resources` at all: a fresh dictionary is correct here.
                None => Some(
                    doc.add_object(Object::Dictionary(lopdf::Dictionary::from_iter(vec![(
                        "Font",
                        Object::Dictionary(lopdf::Dictionary::from_iter(vec![(
                            "F1",
                            Object::Reference(font_ref_id),
                        )])),
                    )]))),
                ),
            };

            if let Some(page_obj) = doc.objects.get_mut(&page_id)
                && let Ok(dict) = page_obj.as_dict_mut()
            {
                dict.set("Contents", Object::Array(all_contents));
                if let Some(res_id) = merged_res {
                    dict.set("Resources", Object::Reference(res_id));
                }
            }
        }

        // Serialize to memory, then write atomically. `doc.save` streams
        // straight into the destination, truncating it first.
        let mut bytes = Vec::new();
        doc.save_to(&mut bytes)
            .map_err(|e| PdfError::IoError(e.to_string()))?;
        write_pdf_output(output_path, &bytes)?;

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
        write_pdf_output(output_path, &pdf_bytes)?;
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
        let doc_bytes = read_input_capped(path)?;
        // Expect cert_path to be the PKCS#8 private key DER, and
        // cert_path with ".x509" extension to be the DER-encoded X.509 cert.
        let key_bytes = std::fs::read(cert_path).map_err(|e| PdfError::IoError(e.to_string()))?;

        // The certificate must be a real X.509 DER blob. The previous code fell
        // back to `key_bytes` when no `.x509` sidecar was found, on the premise
        // that a self-signed key doubles as its own certificate. It does not: a
        // PKCS#8 private key and an X.509 certificate are distinct DER
        // structures, and even a self-signed certificate ships as its own file.
        //
        // Handing `sign()` the private key as the certificate either errors or,
        // worse, produces a signature carrying a certificate no verifier can
        // ever validate -- while `sign_with_certificate` returned `Ok` and the UI
        // reported a successful signature. A missing sidecar is a user error
        // that must be named, not papered over.
        let cert_der_path = std::path::Path::new(cert_path).with_extension("x509");
        let cert_der_bytes = std::fs::read(&cert_der_path).map_err(|e| {
            PdfError::IoError(format!(
                "the signing certificate is missing. Expected the X.509 DER certificate \
                 beside the private key at {}. ({e})",
                cert_der_path.display()
            ))
        })?;

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
        write_pdf_output(output_path, &signed_bytes)?;
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
        let doc_bytes = read_input_capped(path)?;

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
        write_pdf_output(output_path, &buf.into_inner())?;
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
        &mut self,
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

        let engine = get_or_load_ocr_engine(script)?;

        let render_opts = RenderOptions {
            scale: 2.0,
            rotation: 0,
            filter: RenderFilter::None,
            auto_crop: false,
            quality: RenderQuality::High,
        };
        let rendered = self.render_page_internal(doc_id, page_num, render_opts, false)?;

        let page_rect = page.effective_box();
        let page_box = (
            page_rect.x0 as f32,
            page_rect.y0 as f32,
            page_rect.x1 as f32,
            page_rect.y1 as f32,
        );

        let img_source =
            ocrs::ImageSource::from_bytes(&rendered.data, (rendered.width, rendered.height))
                .map_err(|e| {
                    PdfError::EngineError(EngineErrorKind::Generic(format!(
                        "Invalid image for OCR: {e:?}"
                    )))
                })?;
        let ocr_input = engine.prepare_input(img_source).map_err(|e| {
            PdfError::EngineError(EngineErrorKind::Generic(format!(
                "OCR prepare input failed: {e}"
            )))
        })?;
        let word_rects = engine.detect_words(&ocr_input).map_err(|e| {
            PdfError::EngineError(EngineErrorKind::Generic(format!(
                "OCR word detection failed: {e}"
            )))
        })?;
        let line_rects = engine.find_text_lines(&ocr_input, &word_rects);
        let line_texts = engine
            .recognize_text(&ocr_input, &line_rects)
            .map_err(|e| {
                PdfError::EngineError(EngineErrorKind::Generic(format!(
                    "OCR recognition failed: {e}"
                )))
            })?;

        let mut lines: Vec<crate::ocr::OcrLine> = Vec::new();
        for maybe_line in &line_texts {
            let Some(line) = maybe_line else { continue };
            let line_text = line.to_string();
            let trimmed = line_text.trim();
            if trimmed.is_empty() {
                continue;
            }

            let mut words = Vec::new();
            for word in line.words() {
                let w_text = word.to_string();
                let rect = word.bounding_rect();
                // The bitmap is the page's *effective* box with `/Rotate` baked
                // in, and its axes swap on a quarter turn, so the mapping is not
                // a plain two-axis scale of the unrotated box.
                let (x0, y0, x1, y1) = rotated_bitmap_rect_to_page_space(
                    (
                        rect.left() as f32,
                        rect.top() as f32,
                        rect.right() as f32,
                        rect.bottom() as f32,
                    ),
                    (rendered.width, rendered.height),
                    page_box,
                    page.rotate,
                );
                words.push(crate::ocr::OcrWord {
                    text: w_text,
                    bbox: [x0, y0, x1, y1],
                });
            }

            let (lx0, ly0, lx1, ly1) = if words.is_empty() {
                (50.0, 50.0, 500.0, 62.0)
            } else {
                let min_x = words
                    .iter()
                    .map(|w| w.bbox[0])
                    .fold(f32::INFINITY, f32::min);
                let min_y = words
                    .iter()
                    .map(|w| w.bbox[1])
                    .fold(f32::INFINITY, f32::min);
                let max_x = words
                    .iter()
                    .map(|w| w.bbox[2])
                    .fold(f32::NEG_INFINITY, f32::max);
                let max_y = words
                    .iter()
                    .map(|w| w.bbox[3])
                    .fold(f32::NEG_INFINITY, f32::max);
                (min_x, min_y, max_x, max_y)
            };

            lines.push(crate::ocr::OcrLine {
                text: trimmed.to_string(),
                words,
                bbox: [lx0, ly0, lx1, ly1],
            });
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
    ///
    /// Despite the name this is sequential: the OCR engine is a single shared
    /// instance, so pages cannot be recognised concurrently. The name is kept
    /// for API compatibility.
    ///
    /// Fails loudly. This used to swallow every per-page error and return
    /// `Ok(vec![])` when *all* of them failed -- which is exactly what happens
    /// when the OCR models are not installed, since `get_or_load_ocr_engine`
    /// returns `Err` for every page. A caller could not distinguish "this
    /// document has no text" from "nothing ran", and a missing document yielded
    /// `Ok(vec![])` rather than `DocumentNotFound`.
    pub fn ocr_document_parallel(
        &mut self,
        doc_id: DocumentId,
        script: crate::ocr::OcrScript,
    ) -> PdfResult<Vec<crate::ocr::OcrPageResult>> {
        let page_count = self
            .documents
            .get(&doc_id)
            .map(zpdf::PdfDocument::page_count)
            .ok_or(PdfError::EngineError(EngineErrorKind::DocumentNotFound))?;

        let mut results = Vec::with_capacity(page_count);
        let mut failures: Vec<String> = Vec::new();
        for page_num in 0..page_count {
            match self.ocr_page(doc_id, page_num, script) {
                Ok(res) => results.push(res),
                Err(e) => failures.push(format!("page {}: {e}", page_num + 1)),
            }
        }

        if results.is_empty() {
            // Every page failed. The first error is almost always the real
            // cause (a missing model directory, a missing engine), and repeating
            // it once per page buries it.
            let detail = failures
                .first()
                .map(|f| f.split_once(": ").map_or(f.as_str(), |(_, rest)| rest))
                .unwrap_or("no reason reported")
                .to_string();
            return Err(PdfError::EngineError(EngineErrorKind::Generic(format!(
                "OCR failed on all {page_count} pages: {detail}"
            ))));
        }

        if !failures.is_empty() {
            // Partial success: report the pages that worked *and* say which did
            // not, rather than quietly returning a short list.
            tracing::warn!(
                "Document OCR for {doc_id:?}: {} of {page_count} pages failed: {}",
                failures.len(),
                failures.join("; ")
            );
        }

        tracing::info!(
            "Document OCR finished for doc_id {:?} across {} pages ({} failed)",
            doc_id,
            results.len(),
            failures.len()
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
            rotation: 0,
            auto_crop: false,
            quality: RenderQuality::Medium,
            filter: RenderFilter::None,
        };
        let key2 = RenderKey {
            doc_id,
            page_num: 0,
            scale: 100,
            rotation: 0,
            auto_crop: false,
            quality: RenderQuality::Medium,
            filter: RenderFilter::None,
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
            rotation: 0,
            auto_crop: false,
            quality: RenderQuality::Medium,
            filter: RenderFilter::None,
        };
        let key2 = RenderKey {
            doc_id,
            page_num: 1,
            scale: 100,
            rotation: 0,
            auto_crop: false,
            quality: RenderQuality::Medium,
            filter: RenderFilter::None,
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
            rotation: 0,
            auto_crop: false,
            quality: RenderQuality::Medium,
            filter: RenderFilter::None,
        };
        let key2 = RenderKey {
            doc_id,
            page_num: 0,
            scale: 200,
            rotation: 0,
            auto_crop: false,
            quality: RenderQuality::Medium,
            filter: RenderFilter::None,
        };
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_render_key_different_documents() {
        let key1 = RenderKey {
            doc_id: DocumentId(1),
            page_num: 0,
            scale: 100,
            rotation: 0,
            auto_crop: false,
            quality: RenderQuality::Medium,
            filter: RenderFilter::None,
        };
        let key2 = RenderKey {
            doc_id: DocumentId(2),
            page_num: 0,
            scale: 100,
            rotation: 0,
            auto_crop: false,
            quality: RenderQuality::Medium,
            filter: RenderFilter::None,
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
            rotation: 0,
            auto_crop: false,
            quality: RenderQuality::Medium,
            filter: RenderFilter::None,
        };
        let key_high = RenderKey {
            doc_id,
            page_num: 0,
            scale: 200,
            rotation: 0,
            auto_crop: false,
            quality: RenderQuality::Medium,
            filter: RenderFilter::None,
        };
        assert_ne!(key_low, key_high);
    }

    #[test]
    fn test_search_spans_handles_case_insensitive_matching() {
        // `search_spans` with case_sensitive=false should find matches regardless
        // of case, and return accurate per-character bounds.
        let spans = vec![zpdf::TextSpan {
            text: "Hello World".to_string(),
            x: 10.0,
            y: 100.0,
            size: 12.0,
            advance: 60.0,
            mcid: None,
        }];
        let hits = zpdf::search_spans(&spans, "world", false);
        assert_eq!(hits.len(), 1, "case-insensitive search should find 'world'");
        assert_eq!(hits[0].line, "Hello World");
        assert_eq!(hits[0].start, 6);
        assert_eq!(hits[0].len, 5);
    }

    #[test]
    fn test_search_spans_returns_empty_for_empty_query() {
        let spans = vec![zpdf::TextSpan {
            text: "Hello".to_string(),
            x: 0.0,
            y: 100.0,
            size: 12.0,
            advance: 50.0,
            mcid: None,
        }];
        let hits = zpdf::search_spans(&spans, "", false);
        assert!(hits.is_empty(), "empty query should return no hits");
    }

    #[test]
    fn test_search_spans_case_sensitive_distinction() {
        let spans = vec![zpdf::TextSpan {
            text: "Hello World".to_string(),
            x: 0.0,
            y: 100.0,
            size: 12.0,
            advance: 60.0,
            mcid: None,
        }];
        let hits = zpdf::search_spans(&spans, "world", true);
        assert!(
            hits.is_empty(),
            "case-sensitive search should not find 'world' in 'Hello World'"
        );
        let hits = zpdf::search_spans(&spans, "World", true);
        assert_eq!(hits.len(), 1, "case-sensitive search should find 'World'");
    }

    #[test]
    fn test_render_cache_creation() {
        let cache = RenderCache::new(10, 100);
        assert_eq!(
            cache.get(&RenderKey {
                doc_id: DocumentId(1),
                page_num: 0,
                scale: 100,
                rotation: 0,
                auto_crop: false,
                quality: RenderQuality::Medium,
                filter: RenderFilter::None,
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
            rotation: 0,
            auto_crop: false,
            quality: RenderQuality::Medium,
            filter: RenderFilter::None,
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
    fn test_render_started_before_an_invalidation_is_not_cached() {
        // Regression: every worker shares one `RenderCache` but holds its own
        // document state. `ToggleLayer` is broadcast to all of them, so a worker
        // that had not yet dequeued the broadcast still held the pre-toggle
        // layer state: it rendered, then called `put` *after* another worker had
        // already invalidated. `invalidate_document` can only remove keys present
        // at the instant it runs, so the stale bitmap survived and toggling a
        // layer off appeared to do nothing.
        //
        // A generation captured at render start closes that window: the late
        // `put_if_current` notices the document changed and declines to store.
        let cache = RenderCache::new(10, 1024);
        let doc = DocumentId(7);
        let key = RenderKey {
            doc_id: doc,
            page_num: 0,
            scale: 100,
            rotation: 0,
            auto_crop: false,
            quality: RenderQuality::Medium,
            filter: RenderFilter::None,
        };
        let result = crate::models::RenderResult {
            width: 10,
            height: 10,
            data: vec![0u8; 10].into(),
        };

        // A render starts, capturing the current generation.
        let started_at = cache.generation(doc);

        // Meanwhile another worker processes the toggle.
        cache.invalidate_document(doc);

        // The in-flight render now finishes. It must not repopulate the cache.
        let stored = cache.put_if_current(key.clone(), result, started_at);
        assert!(!stored, "a stale render was cached after invalidation");
        assert!(
            cache.get(&key).is_none(),
            "the stale bitmap is reachable by a later request"
        );
    }

    #[test]
    fn test_render_after_an_invalidation_is_still_cached() {
        // The generation guard must not disable caching outright: a render that
        // starts *after* the invalidation is current and should be stored.
        let cache = RenderCache::new(10, 1024);
        let doc = DocumentId(8);
        let key = RenderKey {
            doc_id: doc,
            page_num: 0,
            scale: 100,
            rotation: 0,
            auto_crop: false,
            quality: RenderQuality::Medium,
            filter: RenderFilter::None,
        };
        let result = crate::models::RenderResult {
            width: 10,
            height: 10,
            data: vec![0u8; 10].into(),
        };

        cache.invalidate_document(doc);
        let started_at = cache.generation(doc);
        assert!(cache.put_if_current(key.clone(), result, started_at));
        assert!(cache.get(&key).is_some());
    }

    #[test]
    fn test_tracked_render_keys_stay_bounded_per_document() {
        // Regression: `doc_keys` retained every key ever rendered for the life
        // of the document -- the same unbounded growth its own removed-`Vec`
        // comment claimed to have fixed. Continuous wheel zoom mints a distinct
        // key per quantised scale crossed with rotation/filter/quality, and
        // `quick_cache` evicts without notifying `doc_keys`.
        let cache = RenderCache::new(4096, 64 * 1024 * 1024);
        let doc = DocumentId(9);

        for i in 0..(MAX_TRACKED_KEYS_PER_DOC * 4) {
            let key = RenderKey {
                doc_id: doc,
                page_num: (i % 40),
                scale: 50 + (i % 400) as u32,
                rotation: (i % 4) as i32,
                auto_crop: false,
                quality: RenderQuality::Medium,
                filter: RenderFilter::None,
            };
            cache.put(
                key,
                crate::models::RenderResult {
                    width: 2,
                    height: 2,
                    data: vec![0u8; 4].into(),
                },
            );
        }

        let tracked = cache.tracked_key_count(doc).unwrap_or(0);
        assert!(
            tracked <= MAX_TRACKED_KEYS_PER_DOC,
            "tracked {tracked} keys for one document, cap is {MAX_TRACKED_KEYS_PER_DOC}"
        );
    }

    #[test]
    fn test_closing_a_document_releases_its_key_index() {
        // `tracked_document_count` must return to zero after a close, otherwise
        // the per-document index grows for the life of the process.
        let cache = RenderCache::new(64, 8 * 1024 * 1024);
        let docs: Vec<DocumentId> = (0..8).map(DocumentId).collect();
        for &doc in &docs {
            cache.put(
                RenderKey {
                    doc_id: doc,
                    page_num: 0,
                    scale: 100,
                    rotation: 0,
                    auto_crop: false,
                    quality: RenderQuality::Low,
                    filter: RenderFilter::None,
                },
                crate::models::RenderResult {
                    width: 2,
                    height: 2,
                    data: vec![0u8; 4].into(),
                },
            );
        }
        assert_eq!(cache.tracked_document_count(), docs.len());

        for &doc in &docs {
            cache.remove_document(doc);
        }
        assert_eq!(cache.tracked_document_count(), 0);
    }

    #[test]
    fn test_render_cache_overwrite() {
        let cache = RenderCache::new(10, 1024);
        let key = RenderKey {
            doc_id: DocumentId(1),
            page_num: 0,
            scale: 100,
            rotation: 0,
            auto_crop: false,
            quality: RenderQuality::Medium,
            filter: RenderFilter::None,
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
            rotation: 0,
            auto_crop: false,
            quality: RenderQuality::Medium,
            filter: RenderFilter::None,
        };
        let key2 = RenderKey {
            doc_id: DocumentId(1),
            page_num: 1,
            scale: 100,
            rotation: 0,
            auto_crop: false,
            quality: RenderQuality::Medium,
            filter: RenderFilter::None,
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
                    rotation: 0,
                    auto_crop: false,
                    quality: RenderQuality::Medium,
                    filter: RenderFilter::None,
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

    #[test]
    fn test_oc_config_overrides() {
        let oc = zpdf::OcConfig::default();
        let target_id = zpdf::ObjectId(42, 0);
        assert!(oc.group_visible(target_id));

        let groups = [target_id];
        let mut overrides = std::collections::HashMap::new();
        overrides.insert(target_id, false);
        let oc = apply_oc_overrides(&oc, &overrides, &groups);
        assert!(!oc.group_visible(target_id));

        let mut overrides2 = std::collections::HashMap::new();
        overrides2.insert(target_id, true);
        let oc = apply_oc_overrides(&oc, &overrides2, &groups);
        assert!(oc.group_visible(target_id));
    }

    #[test]
    fn test_oc_override_preserves_untouched_document_layer_states() {
        // Regression: `apply_oc_overrides` rebuilt `/OCProperties /D` from the
        // overrides map alone, with a hard-coded `/BaseState /ON`, using the
        // document's real config only as an error fallback. A document that hid
        // layers by default therefore opened correctly, but the first toggle of
        // any *other* layer produced a config that no longer listed those in
        // `/OFF` -- so they all became visible. One toggle revealed every layer
        // the author had hidden.
        //
        // Built by round-tripping through a synthetic catalog, since
        // `OcConfig`'s fields are private.
        let hidden_a = zpdf::ObjectId(3, 0);
        let hidden_b = zpdf::ObjectId(4, 0);
        let toggled = zpdf::ObjectId(5, 0);
        let all = [hidden_a, hidden_b, toggled];

        // A base config that hides two of the three groups.
        let base = apply_oc_overrides(
            &zpdf::OcConfig::default(),
            &{
                let mut m = std::collections::HashMap::new();
                m.insert(hidden_a, false);
                m.insert(hidden_b, false);
                m
            },
            &all,
        );
        assert!(!base.group_visible(hidden_a));
        assert!(!base.group_visible(hidden_b));
        assert!(base.group_visible(toggled));

        // Now toggle the *third* layer, leaving the other two alone.
        let after = apply_oc_overrides(
            &base,
            &{
                let mut m = std::collections::HashMap::new();
                m.insert(toggled, false);
                m
            },
            &all,
        );

        assert!(
            !after.group_visible(hidden_a),
            "toggling one layer revealed a layer the document had hidden"
        );
        assert!(
            !after.group_visible(hidden_b),
            "toggling one layer revealed a layer the document had hidden"
        );
        assert!(!after.group_visible(toggled));
    }

    #[test]
    fn test_empty_overrides_return_the_document_config_unchanged() {
        // The pass-through path is the only one that can preserve a document's
        // exact `/BaseState`, so it must not be routed through the synthesizer.
        let base = zpdf::OcConfig::default();
        let out = apply_oc_overrides(&base, &std::collections::HashMap::new(), &[]);
        assert!(out.all_visible());
        assert!(out.group_visible(zpdf::ObjectId(1, 0)));
    }

    #[test]
    fn test_validate_conformance_all_profiles() {
        let store = DocumentStore::new(create_render_cache(10, 0));
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests");
        path.push("test_document.pdf");
        let path_str = path.to_str().unwrap();

        let profiles = [
            (
                "pdfa-1b",
                crate::models::ConformanceFamily::PdfA,
                "PDF/A-1b",
            ),
            (
                "pdfa-2b",
                crate::models::ConformanceFamily::PdfA,
                "PDF/A-2b",
            ),
            (
                "pdfa-3b",
                crate::models::ConformanceFamily::PdfA,
                "PDF/A-3b",
            ),
            (
                "pdfx-1a",
                crate::models::ConformanceFamily::PdfX,
                "PDF/X-1a",
            ),
            ("pdfx-3", crate::models::ConformanceFamily::PdfX, "PDF/X-3"),
            ("pdfx-4", crate::models::ConformanceFamily::PdfX, "PDF/X-4"),
            ("pdfx-6", crate::models::ConformanceFamily::PdfX, "PDF/X-6"),
            (
                "pdfua-1",
                crate::models::ConformanceFamily::PdfUa,
                "PDF/UA-1",
            ),
            (
                "pdfua-2",
                crate::models::ConformanceFamily::PdfUa,
                "PDF/UA-2",
            ),
        ];

        for (prof_str, expected_family, expected_name) in profiles {
            let res = store.validate_conformance(path_str, prof_str);
            assert!(
                res.is_ok(),
                "Validation failed for profile {prof_str}: {res:?}"
            );
            let report = res.unwrap();
            assert_eq!(report.family, expected_family);
            assert_eq!(report.profile, expected_name);
        }

        // Verify invalid profile returns an explicit Err rather than silently falling back
        let err_res = store.validate_conformance(path_str, "unknown-standard-99");
        assert!(
            err_res.is_err(),
            "Expected error for invalid profile string"
        );
    }
}
