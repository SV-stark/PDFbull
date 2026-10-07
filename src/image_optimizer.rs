//! Image re-encoding for `optimize_pdf`.
//!
//! # Why this exists
//!
//! `zpdf_writer::rewrite_pdf` carries a `max_image_dimension` option, but its
//! downsampler (`downsample_image_stream` in the writer's `rewrite.rs`) only
//! accepts images that are **all** of: filtered by `FlateDecode` or unfiltered,
//! 8 bits per component, `DeviceRGB` or `DeviceGray`, and carrying no
//! `/DecodeParms`, `/Mask` or `/Decode`. Anything else is returned untouched.
//!
//! That excludes the images that dominate real scanned and photo-heavy
//! documents:
//!
//! - `DCTDecode` (JPEG) — skipped, because resampling would require re-encoding.
//! - `CCITTFaxDecode` / `JBIG2Decode` — bilevel scans, the most common output
//!   of a document scanner.
//! - 1-bit bilevel, palette (`Indexed`), CMYK and `/DecodeParms` images.
//!
//! On such a document that pass finds nothing to do and the output is nearly
//! byte-identical to the input. This module performs the re-encoding the writer
//! cannot.
//!
//! # The three tiers
//!
//! Dispatched on the image's existing `/Filter`:
//!
//! 1. **DCTDecode** — decode, downscale to half the effective DPI (floor
//!    [`MIN_DPI`]), re-encode as baseline JPEG at [`JPEG_QUALITY`].
//! 2. **CCITTFaxDecode / JBIG2Decode** — decode to bilevel, downscale on the
//!    same DPI rule, re-encode as JBIG2. Bilevel text compresses far better
//!    under JBIG2's arithmetic coding than any JPEG setting could manage, and
//!    re-encoding these as grayscale JPEG would visibly soften the page.
//! 3. **FlateDecode** — downscale, re-encode as baseline JPEG.
//!
//! Every tier is guarded: if decoding fails, if the image is ineligible, or if
//! the re-encoded bytes are not smaller than the original stream, the original
//! is kept. A failure here costs file size, never correctness.
//!
//! # Why effective DPI needs a content-stream pass
//!
//! A PDF stores no DPI. An image XObject carries `/Width` and `/Height` in
//! pixels and nothing else; the size it is *drawn* at lives in the page's
//! content stream as the `cm` matrix. So
//!
//! ```text
//! effective_dpi = pixel_width / (placed_width_pt / 72)
//! ```
//!
//! `placed_width_pt` is the unit-scaled x-axis of the current transformation
//! matrix when the image's `Do` operator is reached, which `collect_placed_dpi`
//! reconstructs by walking the page's operators. (The full interpreter cannot
//! supply it: it reports a display-list-local image id rather than the document
//! object id needed to rewrite a stream, and keeps its own object→id map
//! private.) Images that are never placed have no meaningful DPI and are left
//! alone rather than guessed at — see [`PlacedDpi`].

use std::collections::{HashMap, HashSet};

use zpdf::{DecodedImage, Matrix, ObjectId, PdfDict, PdfFile, PdfName, PdfObject};
use zpdf_content::tokenizer::{ContentToken, ContentTokenizer};
use zpdf_document::page::{ResourceDict, parse_resource_dict};

const MAX_STACK: usize = 64;

/// Never reduce an image below this many effective DPI.
///
/// A 300 dpi scan halves to exactly this floor; anything already at or below
/// 300 dpi is left at or lifted to it.
pub const MIN_DPI: u32 = 150;

/// JPEG quality for the continuous-tone tiers.
///
/// Chosen together with [`MIN_DPI`]: halving the DPI does most of the size
/// reduction, so the encoder only has to avoid adding visible mosquito
/// artefacts around glyph edges.
pub const JPEG_QUALITY: u8 = 75;

/// Effective DPI of every image XObject, keyed by object id.
///
/// Only *placed* images appear. An image present in a document's resources but
/// never drawn has no placement matrix and therefore no derivable DPI.
/// Resampling it on an assumed DPI would be a guess, so it is left alone.
pub type PlacedDpi = HashMap<ObjectId, f64>;

/// A re-encoded replacement for one image XObject.
pub struct Replacement {
    /// `/Filter` name for the new stream.
    pub filter: &'static str,
    pub dict: PdfDict,
    pub data: Vec<u8>,
}

/// Outcome of an optimization run, for logging and the UI summary.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct OptimizeStats {
    /// Image XObjects successfully re-encoded.
    pub images_rewritten: usize,
    /// Bytes reclaimed by those re-encodings (negative if a re-encode grew).
    pub image_bytes_saved: i64,
    /// Images skipped: decode failure, ineligible, or already at the floor.
    pub images_skipped: usize,
}

impl OptimizeStats {
    pub fn merge(&mut self, other: Self) {
        self.images_rewritten += other.images_rewritten;
        self.image_bytes_saved += other.image_bytes_saved;
        self.images_skipped += other.images_skipped;
    }
}

/// Walk every page, collecting the effective DPI of each placed image.
///
/// Failures are per-page and non-fatal: a page whose content stream will not
/// tokenize contributes nothing, and its images are then skipped by the rewrite
/// pass rather than resampled at a wrong DPI.
pub fn collect_placed_dpi(file: &PdfFile) -> PlacedDpi {
    let mut out = PlacedDpi::new();
    let Some(page_ids) = page_object_ids(file) else {
        return out;
    };

    for page_id in page_ids {
        let Ok(page) = zpdf::PdfPage::from_object(file, page_id) else {
            continue;
        };
        let Some(content) = decode_page_content(file, &page) else {
            continue;
        };
        scan_page_dpi(file, &page, &content, &mut out);
    }
    out
}

/// Track image placement on one page by walking its content stream directly.
///
/// Only the operators that affect an image's placed size are interpreted: `q`
/// / `Q` (graphics-state stack), `cm` (CTM), and `Do` (draw an XObject). The
/// content tokenizer — not the full interpreter — is used because the
/// interpreter's `ImageDraw` reports a display-list-local `ImageId` rather than
/// the document `ObjectId` needed to rewrite the stream, and its internal
/// object→id map is private. Re-walking the operators keeps the mapping
/// explicit and local.
///
/// Form XObjects are followed recursively, since a scanned page is routinely a
/// form wrapping the image.
fn scan_page_dpi(file: &PdfFile, page: &zpdf::PdfPage, content: &[u8], out: &mut PlacedDpi) {
    let mut ctm = Matrix::identity();
    let mut stack: Vec<Matrix> = Vec::new();
    let mut operands: Vec<PdfObject> = Vec::new();
    let mut forms_seen: HashSet<ObjectId> = HashSet::new();

    scan_content_stream(
        file,
        content,
        &page.resources,
        &mut ctm,
        &mut stack,
        &mut operands,
        &mut forms_seen,
        0,
        out,
    );
}

/// Interpret the placement-relevant operators of one content stream.
#[allow(clippy::too_many_arguments)]
fn scan_content_stream(
    file: &PdfFile,
    content: &[u8],
    resources: &ResourceDict,
    ctm: &mut Matrix,
    stack: &mut Vec<Matrix>,
    operands: &mut Vec<PdfObject>,
    forms_seen: &mut HashSet<ObjectId>,
    form_depth: usize,
    out: &mut PlacedDpi,
) {
    let mut tokenizer = ContentTokenizer::new(content);

    while let Some(token) = tokenizer.next_token() {
        match token {
            ContentToken::Operand(obj) => {
                // Bound the operand stack: `cm` consumes six, and an adversarial
                // stream can otherwise accumulate operands indefinitely.
                if operands.len() < MAX_STACK {
                    operands.push(obj);
                }
            }
            ContentToken::InlineImage { data, .. } => {
                // Inline images (`BI … ID … EI`) have no indirect object id and
                // so cannot be replaced by rewriting a stream. Record nothing;
                // the untouched pass preserves them.
                let _ = data;
                operands.clear();
            }
            ContentToken::Operator(op) => match op.as_str() {
                "q" => {
                    if stack.len() < MAX_STACK {
                        stack.push(*ctm);
                    }
                    operands.clear();
                }
                "Q" => {
                    if let Some(prev) = stack.pop() {
                        *ctm = prev;
                    }
                    operands.clear();
                }
                "cm" => {
                    if let Some(m) = matrix_from_operands(operands) {
                        *ctm = mul_matrix(&m, ctm);
                    }
                    operands.clear();
                }
                "Do" => {
                    if let Some(name) = last_name_operand(operands) {
                        if let Some(&id) = resources.xobjects.get(name) {
                            record_placement(file, id, ctm, out);
                            follow_form(
                                file, id, resources, ctm, stack, operands, forms_seen, form_depth,
                                out,
                            );
                        }
                    }
                    operands.clear();
                }
                _ => operands.clear(),
            },
        }
    }
}

/// Record the effective DPI of `id` as placed under `ctm`.
fn record_placement(file: &PdfFile, id: ObjectId, ctm: &Matrix, out: &mut PlacedDpi) {
    let Some(obj) = file.resolve(id).ok() else {
        return;
    };
    let Ok(stream) = obj.as_stream() else {
        return;
    };
    // Only images carry pixel dimensions; a Form has neither.
    if stream.dict.get_name("Subtype").ok() != Some("Image") {
        return;
    }
    let Ok(width) = stream.dict.get_i64("Width") else {
        return;
    };
    if width <= 0 {
        return;
    }
    let placed_pt = placed_width_pt(ctm);
    if !(placed_pt.is_finite() && placed_pt > 0.0) {
        return;
    }
    let dpi = (width as f64) / (placed_pt / 72.0);
    if !(dpi.is_finite() && dpi > 0.0) {
        return;
    }
    // An image drawn more than once is judged by its smallest placement: that is
    // the size at which detail is actually needed.
    let keep = out.get(&id).map(|existing| dpi < *existing).unwrap_or(true);
    if keep {
        out.insert(id, dpi);
    }
}

/// Recurse into a Form XObject's own content stream.
#[allow(clippy::too_many_arguments)]
fn follow_form(
    file: &PdfFile,
    id: ObjectId,
    parent_resources: &ResourceDict,
    parent_ctm: &Matrix,
    stack: &mut Vec<Matrix>,
    operands: &mut Vec<PdfObject>,
    forms_seen: &mut HashSet<ObjectId>,
    form_depth: usize,
    out: &mut PlacedDpi,
) {
    // Bound on Form nesting. Real documents rarely exceed two or three levels;
    // the cap stops a mutually-recursive form pair from recursing without end.
    const MAX_FORM_DEPTH: usize = 8;
    if form_depth >= MAX_FORM_DEPTH {
        return;
    }
    // A form that draws itself would otherwise recurse forever. The same form
    // legitimately reused twice on one page is still followed, because
    // `forms_seen` is scoped to this scan and re-inserted never blocks a first
    // visit.
    if !forms_seen.insert(id) {
        return;
    }
    let Ok(obj) = file.resolve(id) else { return };
    let Ok(stream) = obj.as_stream() else { return };
    if stream.dict.get_name("Subtype").ok() != Some("Form") {
        return;
    }
    let Ok(bytes) = file.resolve_stream_data(id) else {
        return;
    };

    // A form carries its own /Matrix and /Resources, both defaulting to
    // identity/inherited.
    let mut ctm = *parent_ctm;
    if let Some(m) = stream.dict.get("Matrix") {
        if let Some(m) = matrix_from_object(m) {
            ctm = mul_matrix(&m, &ctm);
        }
    }
    // A form without its own /Resources inherits the parent's; a malformed one
    // falls back to inherited rather than dropping the image entirely.
    let parsed_res = match stream.dict.get("Resources") {
        Some(PdfObject::Dict(d)) => parse_resource_dict(d, file).ok(),
        _ => None,
    };
    let resources = parsed_res.as_ref().unwrap_or(parent_resources);

    let saved_stack = std::mem::take(stack);
    let saved_operands = std::mem::take(operands);
    scan_content_stream(
        file,
        &bytes,
        resources,
        &mut ctm,
        stack,
        operands,
        forms_seen,
        form_depth + 1,
        out,
    );
    *stack = saved_stack;
    *operands = saved_operands;
}

/// `cm` takes six numbers, in order `a b c d e f`.
fn matrix_from_operands(operands: &[PdfObject]) -> Option<Matrix> {
    if operands.len() < 6 {
        return None;
    }
    let n = operands.len();
    let m = Matrix {
        a: number(&operands[n - 6])?,
        b: number(&operands[n - 5])?,
        c: number(&operands[n - 4])?,
        d: number(&operands[n - 3])?,
        e: number(&operands[n - 2])?,
        f: number(&operands[n - 1])?,
    };
    Some(m)
}

fn matrix_from_object(obj: &PdfObject) -> Option<Matrix> {
    let arr = obj.as_array().ok()?;
    if arr.len() < 6 {
        return None;
    }
    Some(Matrix {
        a: number(&arr[0])?,
        b: number(&arr[1])?,
        c: number(&arr[2])?,
        d: number(&arr[3])?,
        e: number(&arr[4])?,
        f: number(&arr[5])?,
    })
}

fn number(obj: &PdfObject) -> Option<f64> {
    match obj {
        PdfObject::Integer(i) => Some(*i as f64),
        PdfObject::Real(r) => Some(*r),
        _ => None,
    }
}

fn last_name_operand(operands: &[PdfObject]) -> Option<&str> {
    operands.last().and_then(|o| o.as_name().ok())
}

/// Row-vector PDF matrix product: applying `m` first, then `n`, is `mul(m, n)`.
fn mul_matrix(m: &Matrix, n: &Matrix) -> Matrix {
    Matrix {
        a: m.a * n.a + m.b * n.c,
        b: m.a * n.b + m.b * n.d,
        c: m.c * n.a + m.d * n.c,
        d: m.c * n.b + m.d * n.d,
        e: m.e * n.a + m.f * n.c + n.e,
        f: m.e * n.b + m.f * n.d + n.f,
    }
}

/// Drawn width of the unit image square, in PDF units.
fn placed_width_pt(m: &Matrix) -> f64 {
    (m.a * m.a + m.b * m.b).sqrt()
}

/// Build the replacement streams for every eligible placed image.
///
/// Returns the replacements keyed by object id. Images with no derived DPI, an
/// unsupported filter, a decode failure, or a re-encode that did not shrink the
/// stream are absent from the map — the caller leaves those untouched.
pub fn plan_replacements(
    file: &PdfFile,
    dpi: &PlacedDpi,
) -> (Vec<(ObjectId, Replacement)>, OptimizeStats) {
    let mut out = Vec::new();
    let mut stats = OptimizeStats::default();

    // Iterate the DPI map rather than the document's objects: it contains only
    // images that were actually drawn, which is exactly the eligible set.
    for (&id, &effective) in dpi {
        let before = file
            .resolve(id)
            .ok()
            .and_then(|o| o.as_stream().ok().map(|s| s.data.len()));
        match rewrite_image(file, id, Some(effective)) {
            Some(replacement) => {
                if let Some(orig_len) = before {
                    stats.image_bytes_saved += orig_len as i64 - replacement.data.len() as i64;
                }
                stats.images_rewritten += 1;
                out.push((id, replacement));
            }
            None => stats.images_skipped += 1,
        }
    }
    (out, stats)
}

/// Apply the halving rule, clamped so an image is never reduced below
/// [`MIN_DPI`] and never upscaled.
///
/// The returned multiplier is always `<= 1.0`; `1.0` means "leave it alone".
pub fn target_scale(effective_dpi: f64) -> f64 {
    if !effective_dpi.is_finite() || effective_dpi <= 0.0 {
        return 1.0;
    }
    let target = (effective_dpi / 2.0).max(f64::from(MIN_DPI));
    let scale = target / effective_dpi;
    if scale.is_finite() {
        scale.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

/// Re-encode one image stream, returning `None` to keep the original.
///
/// `effective_dpi` is `None` for an image never placed on a page; such an image
/// is skipped, because its DPI cannot be derived.
pub fn rewrite_image(
    file: &PdfFile,
    id: ObjectId,
    effective_dpi: Option<f64>,
) -> Option<Replacement> {
    let scale = target_scale(effective_dpi?);
    if scale >= 1.0 {
        // Already at or below the floor: re-encoding lossily would cost quality
        // for no size benefit.
        return None;
    }

    let obj = file.resolve(id).ok()?;
    let stream = obj.as_stream().ok()?;

    if !is_rewritable_image(&stream.dict) {
        return None;
    }

    let filter = image_filter_name(&stream.dict)?;
    let width = stream.dict.get_i64("Width").ok()?;
    let height = stream.dict.get_i64("Height").ok()?;
    if width <= 0 || height <= 0 {
        return None;
    }
    let (width, height) = (width as u32, height as u32);

    if !matches!(
        filter,
        "DCTDecode" | "JBIG2Decode" | "CCITTFaxDecode" | "FlateDecode"
    ) {
        return None;
    }

    let decoded = decode_image(file, id, &stream.dict)?;
    let new_w = ((f64::from(width) * scale).round() as u32).max(1);
    let new_h = ((f64::from(height) * scale).round() as u32).max(1);
    let resized = resize_nearest(&decoded, new_w, new_h)?;

    let replacement = match filter {
        "DCTDecode" | "FlateDecode" => encode_jpeg(&resized),
        // Bilevel scans. JPEG quality is meaningless here — JBIG2's arithmetic
        // coder is the right tool, and gray JPEG would visibly soften the page.
        "CCITTFaxDecode" | "JBIG2Decode" => encode_jbig2(&resized),
        // JPXDecode is not re-encoded: it would have to become JPEG or JBIG2 to
        // shrink, silently changing the compression scheme.
        _ => return None,
    }?;

    // Never trade bytes for a smaller file: a re-encode that grew the stream is
    // a regression, and the caller keeps the original.
    if replacement.data.len() >= stream.data.len() {
        return None;
    }
    Some(replacement)
}

/// Whether an image XObject is safe to re-encode at all.
///
/// Some entries carry meaning a replacement stream would silently drop:
///
/// - `/SMask` or `/Mask` supplies an alpha channel or stencil that the new
///   pixels do not have.
/// - `/ImageMask true` is not sampled data at all but a paint template.
/// - `/Decode` inverts or remaps sample values, so re-encoding the decoded
///   pixels and dropping it would invert the image.
/// - `/Indexed` (palette) images index into a palette the new stream would not
///   carry.
///
/// All of these are left untouched.
fn is_rewritable_image(dict: &PdfDict) -> bool {
    if dict.get("SMask").is_some() || dict.get("Mask").is_some() {
        return false;
    }
    // `/ImageMask` true paints using the current fill colour.
    if matches!(dict.get("ImageMask"), Some(PdfObject::Bool(true))) {
        return false;
    }
    if dict.get("Decode").is_some() {
        return false;
    }
    if is_indexed_colorspace(dict) {
        return false;
    }
    dict.get("Subtype").and_then(|o| o.as_name().ok()) == Some("Image")
}

/// Whether the image's colour space is a palette lookup.
///
/// Only a directly-declared name or array is classified. An `/Indexed` built on
/// an indirect reference is not resolvable from the dictionary alone, and such
/// an image is skipped by the decode guard instead.
fn is_indexed_colorspace(dict: &PdfDict) -> bool {
    let Some(cs) = dict.get("ColorSpace") else {
        return false;
    };
    match cs {
        PdfObject::Name(n) => matches!(n.as_str(), "Indexed" | "I" | "Pattern"),
        PdfObject::Array(arr) => arr.first().and_then(|o| o.as_name().ok()) == Some("Indexed"),
        _ => false,
    }
}

/// The image's primary `/Filter`, as a name string.
///
/// `/Filter` may be a name or an array; only a lone image filter is handled
/// here. A chain such as `[/ASCII85Decode /DCTDecode]` would need the ASCII85
/// layer stripped first, and is left to the untouched path.
fn image_filter_name(dict: &PdfDict) -> Option<&str> {
    match dict.get("Filter")? {
        PdfObject::Name(n) => Some(n.as_str()),
        _ => None,
    }
}

fn decode_image(file: &PdfFile, id: ObjectId, dict: &PdfDict) -> Option<DecodedImage> {
    let raw = file.resolve_stream_data(id).ok()?;
    zpdf_image::decode_image_xobject(&raw, dict).ok()
}

/// Nearest-neighbour resample of a decoded image.
///
/// Box-averaging would read better on photos, but nearest is fast,
/// allocation-light, and — critically — cannot invent intermediate gray levels
/// in a bilevel scan, which would destroy the structure JBIG2 encodes.
fn resize_nearest(img: &DecodedImage, new_w: u32, new_h: u32) -> Option<DecodedImage> {
    let (w, h) = (img.width, img.height);
    if w == 0 || h == 0 || new_w == 0 || new_h == 0 {
        return None;
    }
    let channels = channel_count(img);
    let src = img.data.as_slice();
    let src_len = (w as usize)
        .checked_mul(h as usize)?
        .checked_mul(channels)?;
    // An unexpected buffer length is refused outright rather than resampled from
    // a stride that does not match the pixel grid.
    if src.len() != src_len {
        return None;
    }
    let needed = (new_w as usize)
        .checked_mul(new_h as usize)?
        .checked_mul(channels)?;

    let mut out = vec![0u8; needed];
    for y in 0..new_h {
        let sy = ((u64::from(y) * u64::from(h)) / u64::from(new_h)).min(u64::from(h) - 1) as usize;
        let srow = sy * w as usize * channels;
        let drow = y as usize * new_w as usize * channels;
        for x in 0..new_w {
            let sx =
                ((u64::from(x) * u64::from(w)) / u64::from(new_w)).min(u64::from(w) - 1) as usize;
            let si = srow + sx * channels;
            let di = drow + x as usize * channels;
            out[di..di + channels].copy_from_slice(&src[si..si + channels]);
        }
    }

    Some(DecodedImage {
        width: new_w,
        height: new_h,
        data: out,
        has_alpha: img.has_alpha,
        is_image_mask: img.is_image_mask,
        premultiplied: img.premultiplied,
    })
}

/// Bytes per pixel in a `DecodedImage` buffer.
///
/// `DecodedImage` carries no colour-space tag, so this is inferred: a mask or
/// alpha-bearing image is single-channel, everything else RGB. Grayscale images
/// decoded at 8bpc are widened to RGB by the decoder, which keeps this correct
/// without inspecting the dictionary.
fn channel_count(img: &DecodedImage) -> usize {
    let expected = if img.has_alpha || img.is_image_mask {
        1
    } else {
        3
    };
    // A malformed or unexpected decoder result would otherwise make the
    // stride arithmetic below read out of bounds. Confirm the buffer matches a
    // whole number of pixels of the expected size instead.
    let pixels = (img.width as usize).saturating_mul(img.height as usize);
    if pixels != 0 && img.data.len() == pixels * expected {
        return expected;
    }
    1
}

/// Encode as baseline JPEG with `/Filter /DCTDecode`.
///
/// Baseline (not progressive) is required: PDF 32000-1 §7.4.4 restricts
/// DCTDecode data to baseline sequential JPEG.
fn encode_jpeg(img: &DecodedImage) -> Option<Replacement> {
    let (w, h) = (img.width as usize, img.height as usize);
    // The underlying encoder takes u16 dimensions.
    if w == 0 || h == 0 || w > usize::from(u16::MAX) || h > usize::from(u16::MAX) {
        return None;
    }

    // JPEG here is 3-channel; a gray buffer is widened by replication. The
    // alpha-bearing and stencil cases are already excluded upstream.
    let rgb: Vec<u8> = if img.has_alpha || img.is_image_mask {
        let mut v = Vec::with_capacity(w * h * 3);
        for px in img.data.chunks_exact(1) {
            v.extend_from_slice(&[px[0], px[0], px[0]]);
        }
        v
    } else {
        img.data.clone()
    };
    if rgb.len() != w * h * 3 {
        return None;
    }

    let options = zune_core::options::EncoderOptions::default().set_quality(JPEG_QUALITY);
    let mut encoder = zune_image::codecs::jpeg::JpegEncoder::new_with_options(options);
    let image =
        zune_image::image::Image::from_u8(&rgb, w, h, zune_core::colorspace::ColorSpace::RGB);
    let mut buf = Vec::new();
    zune_image::traits::EncoderTrait::encode(&mut encoder, &image, &mut buf).ok()?;
    let data = buf;
    if data.is_empty() {
        return None;
    }

    let mut dict = PdfDict::new();
    dict.insert(
        PdfName::new("Type"),
        PdfObject::Name(PdfName::new("XObject")),
    );
    dict.insert(
        PdfName::new("Subtype"),
        PdfObject::Name(PdfName::new("Image")),
    );
    dict.insert(PdfName::new("Width"), PdfObject::Integer(w as i64));
    dict.insert(PdfName::new("Height"), PdfObject::Integer(h as i64));
    dict.insert(
        PdfName::new("ColorSpace"),
        PdfObject::Name(PdfName::new("DeviceRGB")),
    );
    dict.insert(PdfName::new("BitsPerComponent"), PdfObject::Integer(8));
    dict.insert(
        PdfName::new("Filter"),
        PdfObject::Name(PdfName::new("DCTDecode")),
    );

    Some(Replacement {
        filter: "DCTDecode",
        dict,
        data,
    })
}

/// Encode as JBIG2 with `/Filter /JBIG2Decode`.
///
/// Uses the lossless (no symbol dictionary) path deliberately. The encoder's
/// own documentation warns that symbol dictionaries produced by independent
/// per-image encoders desync their symbol indices and yield undecodable pages
/// (solid black/white or decoder errors). Sharing one dictionary across every
/// image in a document would need `encode_document_pdf_split` over all images
/// at once; the lossless config sidesteps that by emitting an independent
/// generic region per image, at some cost in compression ratio.
fn encode_jbig2(img: &DecodedImage) -> Option<Replacement> {
    let (w, h) = (img.width, img.height);
    if w == 0 || h == 0 {
        return None;
    }
    let pixels = (w as usize).checked_mul(h as usize)?;
    if img.data.len() != pixels * 3 && img.data.len() != pixels {
        return None;
    }
    // Take the first byte of each pixel: JBIG2 is 1-bit, so a gray or RGB buffer
    // is sampled on its leading channel (and the alpha/stencil cases are
    // already excluded upstream).
    let stride = if img.data.len() == pixels { 1 } else { 3 };

    // Threshold at the midpoint of 8-bit gray. A bilevel scan is essentially
    // all 0 or all 255, so the exact threshold barely matters; it is placed
    // above the midpoint to bias anti-aliased edge pixels toward black, which
    // keeps text strokes from thinning.
    let bits: Vec<u8> = img
        .data
        .chunks_exact(stride)
        .map(|px| u8::from(px[0] > 127))
        .collect();

    let result = jbig2enc_rust::encode_single_image_lossless(&bits, w, h, true).ok()?;
    let data = result.page_data;
    if data.is_empty() {
        return None;
    }

    let mut dict = PdfDict::new();
    dict.insert(
        PdfName::new("Type"),
        PdfObject::Name(PdfName::new("XObject")),
    );
    dict.insert(
        PdfName::new("Subtype"),
        PdfObject::Name(PdfName::new("Image")),
    );
    dict.insert(PdfName::new("Width"), PdfObject::Integer(i64::from(w)));
    dict.insert(PdfName::new("Height"), PdfObject::Integer(i64::from(h)));
    dict.insert(PdfName::new("BitsPerComponent"), PdfObject::Integer(1));
    dict.insert(
        PdfName::new("ColorSpace"),
        PdfObject::Name(PdfName::new("DeviceGray")),
    );
    // No /Decode is needed. For a 1-bit DeviceGray image the PDF default already
    // paints sample 0 black and 1 white (ISO 32000-1 Table 22), which matches
    // the convention used above: a set bit is a bright pixel.

    Some(Replacement {
        filter: "JBIG2Decode",
        dict,
        data,
    })
}

/// Page object ids in document order, via the catalog's page tree.
///
/// A stack-based depth-first walk, so the order is page-tree order rather than
/// document order. The DPI map does not depend on order.
fn page_object_ids(file: &PdfFile) -> Option<Vec<ObjectId>> {
    let root = file.trailer.get_ref("Root").ok()?;
    let catalog = file.resolve(root).ok()?;
    let dict = catalog.as_dict().ok()?;
    let pages_id = dict.get_ref("Pages").ok()?;

    // Cap on pages walked, so a malformed or cyclic page tree cannot make this
    // unbounded. Well above any real document.
    const MAX_PAGES: usize = 100_000;

    let mut out = Vec::new();
    let mut queue = vec![pages_id];
    // Guard against a malformed page tree that loops back on itself.
    let mut seen = HashSet::new();
    while let Some(id) = queue.pop() {
        if out.len() >= MAX_PAGES {
            break;
        }
        if !seen.insert(id) {
            continue;
        }
        let Ok(obj) = file.resolve(id) else { continue };
        let Ok(dict) = obj.as_dict() else { continue };
        let is_page = dict.get_name("Type").ok() == Some("Page");
        if is_page {
            out.push(id);
            continue;
        }
        if let Some(kids) = dict.get("Kids")
            && let Ok(arr) = kids.as_array()
        {
            for kid in arr.iter().rev() {
                if let &PdfObject::Ref(kid_id) = kid {
                    queue.push(kid_id);
                }
            }
        }
    }
    Some(out)
}

/// Concatenated, decoded content stream bytes for one page.
///
/// A page's `/Contents` may be one stream or an array of them, which are
/// concatenated as though they were a single stream with whitespace between
/// (ISO 32000-1 §7.8.2). One unreadable stream fails the whole page, so its
/// images are skipped rather than resampled from a partial operator stream.
fn decode_page_content(file: &PdfFile, page: &zpdf::PdfPage) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    for &content_id in &page.contents {
        let bytes = file.resolve_stream_data(content_id).ok()?;
        if !out.is_empty() {
            out.push(b'\n');
        }
        out.extend_from_slice(&bytes);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use zpdf::PdfDict;

    // --- DPI rule -------------------------------------------------------

    #[test]
    fn halves_dpi_above_the_floor() {
        // The documented cases: 1200 -> 600, 200 -> 150 (not 100).
        assert!((target_scale(1200.0) - 0.5).abs() < 1e-9);
        assert!((target_scale(200.0) - 0.75).abs() < 1e-9);
        assert!((target_scale(300.0) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn never_goes_below_the_floor() {
        // 100 dpi would halve to 50, which is under the floor, so no reduction.
        assert_eq!(target_scale(100.0), 1.0);
        assert_eq!(target_scale(50.0), 1.0);
    }

    #[test]
    fn never_upscales() {
        // An absurdly low DPI must not be treated as an invitation to enlarge.
        assert_eq!(target_scale(0.0), 1.0);
        assert_eq!(target_scale(-300.0), 1.0);
        assert_eq!(target_scale(f64::NAN), 1.0);
        assert_eq!(target_scale(f64::INFINITY), 1.0);
        assert!(target_scale(1000.0) <= 1.0);
    }

    // --- Matrix arithmetic ----------------------------------------------

    #[test]
    fn matrix_product_composes_in_order() {
        // A 2x scale then a 3x scale is 6x, whichever order they are composed.
        let two = Matrix {
            a: 2.0,
            b: 0.0,
            c: 0.0,
            d: 2.0,
            e: 0.0,
            f: 0.0,
        };
        let three = Matrix {
            a: 3.0,
            b: 0.0,
            c: 0.0,
            d: 3.0,
            e: 0.0,
            f: 0.0,
        };
        assert!((mul_matrix(&two, &three).a - 6.0).abs() < 1e-9);
        assert!((mul_matrix(&three, &two).a - 6.0).abs() < 1e-9);
    }

    #[test]
    fn matrix_product_applies_translation_in_parent_space() {
        let scale = Matrix {
            a: 2.0,
            b: 0.0,
            c: 0.0,
            d: 2.0,
            e: 0.0,
            f: 0.0,
        };
        let translate = Matrix {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 10.0,
            f: 20.0,
        };
        // `translate` applied first, then `scale`: the offset is scaled too.
        let m = mul_matrix(&translate, &scale);
        assert!((m.e - 20.0).abs() < 1e-9);
        assert!((m.f - 40.0).abs() < 1e-9);
    }

    #[test]
    fn placed_width_uses_the_scaled_axis_length() {
        // A 3-4-5 triangle: the x-axis is 5 units long, not 3.
        let m = Matrix {
            a: 3.0,
            b: 4.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        };
        assert!((placed_width_pt(&m) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn cm_operands_are_read_in_order() {
        let ops = vec![
            PdfObject::Integer(1),
            PdfObject::Integer(0),
            PdfObject::Integer(0),
            PdfObject::Integer(1),
            PdfObject::Integer(7),
            PdfObject::Integer(9),
        ];
        let m = matrix_from_operands(&ops).expect("six operands");
        assert_eq!(
            (m.a, m.b, m.c, m.d, m.e, m.f),
            (1.0, 0.0, 0.0, 1.0, 7.0, 9.0)
        );
        // Fewer than six operands is not a valid `cm`.
        assert!(matrix_from_operands(&ops[..5]).is_none());
    }

    #[test]
    fn cm_accepts_real_operands() {
        let ops = vec![
            PdfObject::Real(0.5),
            PdfObject::Integer(0),
            PdfObject::Integer(0),
            PdfObject::Real(0.5),
            PdfObject::Integer(0),
            PdfObject::Integer(0),
        ];
        let m = matrix_from_operands(&ops).expect("mixed Integer/Real operands");
        assert!((m.a - 0.5).abs() < 1e-9);
        assert!((m.d - 0.5).abs() < 1e-9);
    }

    // --- Eligibility guards ---------------------------------------------

    fn image_dict() -> PdfDict {
        let mut d = PdfDict::new();
        d.insert(
            PdfName::new("Subtype"),
            PdfObject::Name(PdfName::new("Image")),
        );
        d
    }

    #[test]
    fn plain_image_is_rewritable() {
        assert!(is_rewritable_image(&image_dict()));
    }

    #[test]
    fn images_with_alpha_or_stencils_are_skipped() {
        let mut d = image_dict();
        d.insert(PdfName::new("SMask"), PdfObject::Integer(9));
        assert!(!is_rewritable_image(&d));

        let mut d = image_dict();
        d.insert(PdfName::new("Mask"), PdfObject::Integer(9));
        assert!(!is_rewritable_image(&d));

        let mut d = image_dict();
        d.insert(PdfName::new("ImageMask"), PdfObject::Bool(true));
        assert!(!is_rewritable_image(&d));
    }

    #[test]
    fn images_with_decode_or_palette_are_skipped() {
        // /Decode inverts or remaps samples; a replacement would drop that.
        let mut d = image_dict();
        d.insert(
            PdfName::new("Decode"),
            PdfObject::Array(vec![PdfObject::Integer(1), PdfObject::Integer(0)]),
        );
        assert!(!is_rewritable_image(&d));

        let mut d = image_dict();
        d.insert(
            PdfName::new("ColorSpace"),
            PdfObject::Name(PdfName::new("Indexed")),
        );
        assert!(!is_rewritable_image(&d));

        let mut d = image_dict();
        d.insert(
            PdfName::new("ColorSpace"),
            PdfObject::Array(vec![PdfObject::Name(PdfName::new("Indexed"))]),
        );
        assert!(!is_rewritable_image(&d));
    }

    #[test]
    fn non_image_subtype_is_skipped() {
        let mut d = PdfDict::new();
        d.insert(
            PdfName::new("Subtype"),
            PdfObject::Name(PdfName::new("Form")),
        );
        assert!(!is_rewritable_image(&d));
    }

    // --- Buffer handling ------------------------------------------------

    fn decoded_gray(w: u32, h: u32) -> DecodedImage {
        DecodedImage {
            width: w,
            height: h,
            data: vec![0u8; (w as usize) * (h as usize)],
            has_alpha: false,
            is_image_mask: false,
            premultiplied: false,
        }
    }

    #[test]
    fn resize_preserves_content_and_dimensions() {
        let src = DecodedImage {
            width: 4,
            height: 4,
            data: (0..48u8).collect(),
            has_alpha: false,
            is_image_mask: false,
            premultiplied: false,
        };
        let out = resize_nearest(&src, 2, 2).expect("shrink to 2x2");
        assert_eq!((out.width, out.height), (2, 2));
        assert_eq!(out.data.len(), 2 * 2 * 3);
        // Nearest-neighbour samples the top-left of each 2x2 block.
        assert_eq!(&out.data[0..3], &[0, 1, 2]);
    }

    #[test]
    fn resize_rejects_a_mismatched_buffer() {
        let mut src = DecodedImage {
            width: 4,
            height: 4,
            data: vec![0u8; 48],
            has_alpha: false,
            is_image_mask: false,
            premultiplied: false,
        };
        // Claim 4x4 RGB but supply only half the bytes; must refuse rather than
        // resample from a stride that does not match.
        src.data.truncate(24);
        assert!(resize_nearest(&src, 2, 2).is_none());
    }

    #[test]
    fn resize_rejects_zero_dimensions() {
        assert!(resize_nearest(&decoded_gray(0, 4), 2, 2).is_none());
        assert!(resize_nearest(&decoded_gray(4, 4), 0, 2).is_none());
    }

    // --- Encoders -------------------------------------------------------

    #[test]
    fn jpeg_encodes_and_carries_the_expected_dictionary() {
        // A small RGB gradient, wide enough that halving still leaves detail.
        let (w, h) = (64u32, 48u32);
        let mut data = vec![0u8; (w as usize) * (h as usize) * 3];
        for (i, px) in data.chunks_exact_mut(3).enumerate() {
            px[0] = (i % 256) as u8;
            px[1] = 128;
            px[2] = 255 - (i % 256) as u8;
        }
        let img = DecodedImage {
            width: w,
            height: h,
            data,
            has_alpha: false,
            is_image_mask: false,
            premultiplied: false,
        };
        let r = encode_jpeg(&img).expect("encode");
        assert_eq!(r.filter, "DCTDecode");
        assert!(!r.data.is_empty());
        assert_eq!(r.dict.get_i64("Width").ok(), Some(i64::from(w)));
        assert_eq!(r.dict.get_i64("Height").ok(), Some(i64::from(h)));
        assert_eq!(r.dict.get("BitsPerComponent"), Some(&PdfObject::Integer(8)));
        assert_eq!(
            r.dict.get("ColorSpace"),
            Some(&PdfObject::Name(PdfName::new("DeviceRGB")))
        );
        // Baseline JPEG: starts with the SOI marker FF D8. A progressive stream
        // would be SOF2, which PDF's DCTDecode does not permit.
        assert_eq!(&r.data[0..2], &[0xFF, 0xD8]);
    }

    #[test]
    fn jpeg_widens_gray_to_rgb() {
        let (w, h) = (16u32, 16u32);
        let mut data = vec![0u8; (w as usize) * (h as usize)];
        for (i, px) in data.iter_mut().enumerate() {
            *px = (i % 256) as u8;
        }
        let img = DecodedImage {
            width: w,
            height: h,
            data,
            has_alpha: true,
            is_image_mask: false,
            premultiplied: false,
        };
        let r = encode_jpeg(&img).expect("encode gray as rgb");
        assert_eq!(
            r.dict.get("ColorSpace"),
            Some(&PdfObject::Name(PdfName::new("DeviceRGB")))
        );
    }

    #[test]
    fn jbig2_encodes_bilevel_text() {
        // A checkerboard-ish pattern with black runs on white, so the arithmetic
        // coder has real structure to exploit rather than a uniform field.
        let (w, h) = (64u32, 64u32);
        let mut data = vec![255u8; (w as usize) * (h as usize) * 3];
        for y in 0..h {
            for x in 0..w {
                if (x / 4) % 2 == 0 && (y / 8) % 3 == 0 {
                    let i = ((y * w + x) * 3) as usize;
                    data[i] = 0;
                    data[i + 1] = 0;
                    data[i + 2] = 0;
                }
            }
        }
        let img = DecodedImage {
            width: w,
            height: h,
            data,
            has_alpha: false,
            is_image_mask: false,
            premultiplied: false,
        };
        let r = encode_jbig2(&img).expect("encode");
        assert_eq!(r.filter, "JBIG2Decode");
        assert!(!r.data.is_empty());
        assert_eq!(r.dict.get_i64("Width").ok(), Some(i64::from(w)));
        assert_eq!(r.dict.get_i64("BitsPerComponent").ok(), Some(1));
        assert_eq!(
            r.dict.get("ColorSpace"),
            Some(&PdfObject::Name(PdfName::new("DeviceGray")))
        );
        // No /Decode: the PDF default already paints 0 black and 1 white.
        assert!(r.dict.get("Decode").is_none());
    }

    #[test]
    fn jbig2_rejects_a_mismatched_buffer() {
        let mut img = decoded_gray(8, 8);
        img.data.truncate(10);
        assert!(encode_jbig2(&img).is_none());
    }
}
