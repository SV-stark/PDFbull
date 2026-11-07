//! Comic books (`.cbz`) and plain images opened like PDFs.
//!
//! The whole viewer — rendering, thumbnails, two-page spreads, zoom, presenting —
//! is built around a PDF engine. Rather than teach every one of those paths a
//! second document model, a comic or image is turned into a small image-only
//! PDF (one page per picture) the first time it is opened, cached on disk, and
//! the cached PDF is what the engine sees. JPEG pages are copied in byte for
//! byte (no recompression, no quality loss); every other format is decoded once
//! and stored losslessly with Flate.
//!
//! The ZIP reader is deliberately tiny: comic archives are plain "stored" or
//! "deflate" ZIPs, so only those two methods are supported. ZIP64 archives,
//! encrypted entries and RAR-based `.cbr` files are refused with a clear error.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Picture formats that can be opened directly, one page per file.
pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "jpe", "webp", "bmp", "qoi"];

/// Comic book archive formats.
pub const COMIC_EXTENSIONS: &[&str] = &["cbz"];

/// Refuse to inflate any single archive entry beyond this, so a crafted
/// "zip bomb" cannot exhaust memory.
const MAX_ENTRY_BYTES: u64 = 128 * 1024 * 1024;

/// Upper bound on the decoded pixel count of one picture (about 100 MP).
const MAX_PIXELS: usize = 100_000_000;

/// Longest page side, in PDF points, after scaling a picture onto a page.
const MAX_PAGE_SIDE_PT: f32 = 1190.0;

/// Pictures are laid out as if printed at this resolution.
const ASSUMED_DPI: f32 = 150.0;

fn ext_of(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
}

fn is_image_name(name: &str) -> bool {
    Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| {
            IMAGE_EXTENSIONS
                .iter()
                .any(|known| e.eq_ignore_ascii_case(known))
        })
}

/// True when `path` names a comic archive or a picture this module converts.
pub fn is_comic_or_image(path: &Path) -> bool {
    ext_of(path).is_some_and(|e| {
        COMIC_EXTENSIONS.contains(&e.as_str()) || IMAGE_EXTENSIONS.contains(&e.as_str())
    })
}

/// Compare two names the way a person reads them, so `page2` sorts before
/// `page10`. Digit runs compare by value; everything else case-insensitively.
pub fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let (mut ai, mut bi) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => return a.cmp(b),
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(ca), Some(cb)) if ca.is_ascii_digit() && cb.is_ascii_digit() => {
                let mut na = String::new();
                while let Some(c) = ai.peek().copied().filter(char::is_ascii_digit) {
                    na.push(c);
                    ai.next();
                }
                let mut nb = String::new();
                while let Some(c) = bi.peek().copied().filter(char::is_ascii_digit) {
                    nb.push(c);
                    bi.next();
                }
                let ta = na.trim_start_matches('0');
                let tb = nb.trim_start_matches('0');
                let ord = ta.len().cmp(&tb.len()).then_with(|| ta.cmp(tb));
                if ord != Ordering::Equal {
                    return ord;
                }
            }
            (Some(ca), Some(cb)) => {
                let la = ca.to_lowercase().next().unwrap_or(ca);
                let lb = cb.to_lowercase().next().unwrap_or(cb);
                if la != lb {
                    return la.cmp(&lb);
                }
                ai.next();
                bi.next();
            }
        }
    }
}

// ── Minimal ZIP reader ──────────────────────────────────────────────────────

fn u16_at(buf: &[u8], off: usize) -> Option<u16> {
    buf.get(off..off + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
}

fn u32_at(buf: &[u8], off: usize) -> Option<u32> {
    buf.get(off..off + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

struct ZipEntry {
    name: String,
    method: u16,
    flags: u16,
    compressed_size: u64,
    uncompressed_size: u64,
    local_header_offset: u64,
}

fn zip_entries(data: &[u8]) -> Result<Vec<ZipEntry>, String> {
    // End of central directory: 22 bytes plus an optional comment of up to 64 KiB.
    const EOCD_SIG: u32 = 0x0605_4b50;
    if data.len() < 22 {
        return Err("the file is too small to be a comic archive".into());
    }
    let search_start = data.len().saturating_sub(22 + 65_535);
    let eocd = (search_start..=data.len() - 22)
        .rev()
        .find(|&i| u32_at(data, i) == Some(EOCD_SIG))
        .ok_or("this is not a ZIP-based comic archive (.cbr/RAR files are not supported)")?;
    let total = u16_at(data, eocd + 10).ok_or("truncated archive")? as usize;
    let cd_offset = u32_at(data, eocd + 16).ok_or("truncated archive")?;
    if cd_offset == u32::MAX || total == u16::MAX as usize {
        return Err("ZIP64 comic archives are not supported yet".into());
    }

    let mut entries = Vec::with_capacity(total);
    let mut off = cd_offset as usize;
    for _ in 0..total {
        if u32_at(data, off) != Some(0x0201_4b50) {
            return Err("the archive's table of contents is damaged".into());
        }
        let flags = u16_at(data, off + 8).ok_or("truncated archive")?;
        let method = u16_at(data, off + 10).ok_or("truncated archive")?;
        let compressed_size = u32_at(data, off + 20).ok_or("truncated archive")? as u64;
        let uncompressed_size = u32_at(data, off + 24).ok_or("truncated archive")? as u64;
        let name_len = u16_at(data, off + 28).ok_or("truncated archive")? as usize;
        let extra_len = u16_at(data, off + 30).ok_or("truncated archive")? as usize;
        let comment_len = u16_at(data, off + 32).ok_or("truncated archive")? as usize;
        let local_header_offset = u32_at(data, off + 42).ok_or("truncated archive")? as u64;
        let name_bytes = data
            .get(off + 46..off + 46 + name_len)
            .ok_or("truncated archive")?;
        let name = String::from_utf8_lossy(name_bytes).replace('\\', "/");
        entries.push(ZipEntry {
            name,
            method,
            flags,
            compressed_size,
            uncompressed_size,
            local_header_offset,
        });
        off += 46 + name_len + extra_len + comment_len;
    }
    Ok(entries)
}

fn zip_read_entry(data: &[u8], entry: &ZipEntry) -> Result<Vec<u8>, String> {
    if entry.flags & 1 != 0 {
        return Err(format!("{} is password-protected", entry.name));
    }
    if entry.uncompressed_size > MAX_ENTRY_BYTES {
        return Err(format!("{} is too large", entry.name));
    }
    let lh = entry.local_header_offset as usize;
    if u32_at(data, lh) != Some(0x0403_4b50) {
        return Err(format!("{} is damaged", entry.name));
    }
    let name_len = u16_at(data, lh + 26).ok_or("truncated archive")? as usize;
    let extra_len = u16_at(data, lh + 28).ok_or("truncated archive")? as usize;
    let start = lh + 30 + name_len + extra_len;
    let end = start
        .checked_add(entry.compressed_size as usize)
        .ok_or("truncated archive")?;
    let raw = data
        .get(start..end)
        .ok_or_else(|| format!("{} is truncated", entry.name))?;
    match entry.method {
        0 => Ok(raw.to_vec()),
        8 => {
            let mut out = Vec::with_capacity(entry.uncompressed_size as usize);
            flate2::read::DeflateDecoder::new(raw)
                .take(MAX_ENTRY_BYTES + 1)
                .read_to_end(&mut out)
                .map_err(|e| format!("{}: {e}", entry.name))?;
            if out.len() as u64 > MAX_ENTRY_BYTES {
                return Err(format!("{} is too large", entry.name));
            }
            Ok(out)
        }
        m => Err(format!(
            "{} uses an unsupported compression method ({m})",
            entry.name
        )),
    }
}

/// The pictures inside a comic archive, in reading order.
fn comic_pages(data: &[u8]) -> Result<Vec<Vec<u8>>, String> {
    let mut entries: Vec<ZipEntry> = zip_entries(data)?
        .into_iter()
        .filter(|e| {
            !e.name.ends_with('/')
                && !e.name.starts_with("__MACOSX/")
                && !e
                    .name
                    .rsplit('/')
                    .next()
                    .is_some_and(|base| base.starts_with('.'))
                && is_image_name(&e.name)
        })
        .collect();
    entries.sort_by(|a, b| natural_cmp(&a.name, &b.name));
    if entries.is_empty() {
        return Err("the archive contains no pictures".into());
    }
    entries.iter().map(|e| zip_read_entry(data, e)).collect()
}

// ── Pictures to PDF ─────────────────────────────────────────────────────────

/// A picture ready to be written as an image XObject.
struct PageImage {
    width: u32,
    height: u32,
    /// 1 (gray) or 3 (RGB).
    components: u8,
    /// `true` for untouched JPEG bytes (DCTDecode), `false` for Flate.
    is_jpeg: bool,
    bytes: Vec<u8>,
}

/// Width, height and component count of a baseline/progressive JPEG.
fn jpeg_info(data: &[u8]) -> Option<(u32, u32, u8)> {
    if data.get(0..2)? != [0xFF, 0xD8] {
        return None;
    }
    let mut i = 2;
    while i + 4 <= data.len() {
        if data[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = data[i + 1];
        if marker == 0xFF {
            i += 1;
            continue;
        }
        if marker == 0xD8 || (0xD0..=0xD7).contains(&marker) || marker == 0x01 {
            i += 2;
            continue;
        }
        let len = u16::from_be_bytes([*data.get(i + 2)?, *data.get(i + 3)?]) as usize;
        let is_sof = matches!(marker, 0xC0..=0xCF) && !matches!(marker, 0xC4 | 0xC8 | 0xCC);
        if is_sof {
            let h = u16::from_be_bytes([*data.get(i + 5)?, *data.get(i + 6)?]) as u32;
            let w = u16::from_be_bytes([*data.get(i + 7)?, *data.get(i + 8)?]) as u32;
            let comps = *data.get(i + 9)?;
            return Some((w, h, comps));
        }
        i += 2 + len;
    }
    None
}

fn prepare_image(data: Vec<u8>) -> Result<PageImage, String> {
    // JPEGs in gray or RGB are embedded verbatim.
    if let Some((w, h, comps)) = jpeg_info(&data)
        && w > 0
        && h > 0
        && (comps == 1 || comps == 3)
    {
        return Ok(PageImage {
            width: w,
            height: h,
            components: comps,
            is_jpeg: true,
            bytes: data,
        });
    }

    use zune_core::bit_depth::BitDepth;
    use zune_core::bytestream::ZCursor;
    use zune_core::colorspace::ColorSpace;
    use zune_core::options::DecoderOptions;

    let options = DecoderOptions::default()
        .set_max_width(50_000)
        .set_max_height(50_000);
    let mut image = zune_image::image::Image::read(ZCursor::new(&data[..]), options)
        .map_err(|e| format!("cannot read picture: {e:?}"))?;
    let (w, h) = image.dimensions();
    if w == 0 || h == 0 || w.saturating_mul(h) > MAX_PIXELS {
        return Err("the picture is empty or too large".into());
    }
    image
        .convert_depth(BitDepth::Eight)
        .map_err(|e| format!("cannot convert picture: {e:?}"))?;
    image
        .convert_color(ColorSpace::RGBA)
        .map_err(|e| format!("cannot convert picture: {e:?}"))?;
    let rgba = image
        .flatten_to_u8()
        .into_iter()
        .next()
        .ok_or("the picture has no frames")?;
    if rgba.len() < w * h * 4 {
        return Err("the picture data is incomplete".into());
    }
    // Composite transparency onto white, the colour of a page.
    let mut rgb = Vec::with_capacity(w * h * 3);
    for px in rgba.as_chunks::<4>().0.iter().take(w * h) {
        let a = px[3] as u32;
        for &c in &px[..3] {
            rgb.push(((c as u32 * a + 255 * (255 - a)) / 255) as u8);
        }
    }
    let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(&rgb).map_err(|e| e.to_string())?;
    let bytes = enc.finish().map_err(|e| e.to_string())?;
    Ok(PageImage {
        width: w as u32,
        height: h as u32,
        components: 3,
        is_jpeg: false,
        bytes,
    })
}

/// Page size in points for a picture of `w`×`h` pixels.
fn page_size_for(w: u32, h: u32) -> (f32, f32) {
    let scale = 72.0 / ASSUMED_DPI;
    let (mut pw, mut ph) = (w as f32 * scale, h as f32 * scale);
    let longest = pw.max(ph);
    if longest > MAX_PAGE_SIDE_PT {
        let k = MAX_PAGE_SIDE_PT / longest;
        pw *= k;
        ph *= k;
    }
    (pw.max(1.0), ph.max(1.0))
}

fn images_to_pdf(images: &[PageImage]) -> Vec<u8> {
    use pdf_writer::{Filter, Finish, Name, Pdf, Rect, Ref};

    let mut pdf = Pdf::new();
    let catalog_id = Ref::new(1);
    let pages_id = Ref::new(2);
    let mut next = 3;
    let mut alloc = || {
        let r = Ref::new(next);
        next += 1;
        r
    };

    let mut page_ids = Vec::with_capacity(images.len());
    for img in images {
        let page_id = alloc();
        let image_id = alloc();
        let content_id = alloc();
        page_ids.push(page_id);

        let (pw, ph) = page_size_for(img.width, img.height);

        let mut xobj = pdf.image_xobject(image_id, &img.bytes);
        xobj.filter(if img.is_jpeg {
            Filter::DctDecode
        } else {
            Filter::FlateDecode
        });
        xobj.width(img.width as i32);
        xobj.height(img.height as i32);
        if img.components == 1 {
            xobj.color_space().device_gray();
        } else {
            xobj.color_space().device_rgb();
        }
        xobj.bits_per_component(8);
        xobj.finish();

        let mut content = pdf_writer::Content::new();
        content.save_state();
        content.transform([pw, 0.0, 0.0, ph, 0.0, 0.0]);
        content.x_object(Name(b"Im0"));
        content.restore_state();
        pdf.stream(content_id, &content.finish());

        let mut page = pdf.page(page_id);
        page.media_box(Rect::new(0.0, 0.0, pw, ph));
        page.parent(pages_id);
        page.contents(content_id);
        page.resources().x_objects().pair(Name(b"Im0"), image_id);
        page.finish();
    }

    pdf.pages(pages_id)
        .kids(page_ids.iter().copied())
        .count(page_ids.len() as i32);
    pdf.catalog(catalog_id).pages(pages_id);
    pdf.finish()
}

/// Build an image-only PDF from a comic archive or a single picture.
pub fn convert_to_pdf(path: &Path) -> Result<Vec<u8>, String> {
    let data =
        crate::pdf_engine::read_input_capped(&path.to_string_lossy()).map_err(|e| e.to_string())?;
    let ext = ext_of(path).unwrap_or_default();
    let raw_pages = if COMIC_EXTENSIONS.contains(&ext.as_str()) {
        comic_pages(&data)?
    } else {
        vec![data]
    };

    use rayon::prelude::*;
    let prepared: Vec<Result<PageImage, String>> =
        raw_pages.into_par_iter().map(prepare_image).collect();
    let mut images = Vec::with_capacity(prepared.len());
    let mut skipped = 0usize;
    for p in prepared {
        match p {
            Ok(img) => images.push(img),
            Err(e) => {
                skipped += 1;
                tracing::warn!("Skipping unreadable picture in {}: {e}", path.display());
            }
        }
    }
    if images.is_empty() {
        return Err("none of the pictures could be read".into());
    }
    if skipped > 0 {
        tracing::warn!(
            "{skipped} picture(s) skipped while opening {}",
            path.display()
        );
    }
    Ok(images_to_pdf(&images))
}

/// Where the converted copy of `path` is kept. The name folds in the source's
/// size and modification time, so editing the comic produces a fresh copy.
pub fn cache_path_for(path: &Path) -> PathBuf {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.to_string_lossy().to_lowercase().hash(&mut hasher);
    if let Ok(meta) = std::fs::metadata(path) {
        meta.len().hash(&mut hasher);
        if let Ok(modified) = meta.modified() {
            modified.hash(&mut hasher);
        }
    }
    let stem: String = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document")
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'))
        .take(60)
        .collect();
    crate::storage::get_config_dir()
        .join("converted")
        .join(format!("{stem}-{:016x}.pdf", hasher.finish()))
}

/// Convert `path` once and return the cached PDF's location.
pub fn ensure_converted(path: &Path) -> Result<PathBuf, String> {
    let out = cache_path_for(path);
    if out.is_file() {
        return Ok(out);
    }
    let pdf = convert_to_pdf(path)?;
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    crate::storage::atomic_write_bytes(&out, &pdf).map_err(|e| e.to_string())?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_order_puts_page2_before_page10() {
        let mut names = vec!["p10.jpg", "p2.jpg", "P1.jpg", "p02b.jpg"];
        names.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(names, vec!["P1.jpg", "p2.jpg", "p02b.jpg", "p10.jpg"]);
    }

    #[test]
    fn recognises_comics_and_images() {
        assert!(is_comic_or_image(Path::new("a/Book.CBZ")));
        assert!(is_comic_or_image(Path::new("scan.jpeg")));
        assert!(!is_comic_or_image(Path::new("doc.pdf")));
        assert!(!is_comic_or_image(Path::new("noext")));
    }

    fn tiny_png() -> Vec<u8> {
        let img = zune_image::image::Image::from_u8(
            &[
                255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 0, 255, 255, 255, 255,
            ],
            2,
            2,
            zune_core::colorspace::ColorSpace::RGBA,
        );
        img.write_to_vec(zune_image::codecs::ImageFormat::PNG)
            .expect("encode png")
    }

    /// A stored (uncompressed) ZIP holding `files`.
    fn stored_zip(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut central = Vec::new();
        for (name, data) in files {
            let offset = out.len() as u32;
            out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
            out.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            out.extend_from_slice(&0u32.to_le_bytes()); // crc (unchecked)
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(data);

            central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
            central.extend_from_slice(&[20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            central.extend_from_slice(&0u32.to_le_bytes());
            central.extend_from_slice(&(data.len() as u32).to_le_bytes());
            central.extend_from_slice(&(data.len() as u32).to_le_bytes());
            central.extend_from_slice(&(name.len() as u16).to_le_bytes());
            central.extend_from_slice(&[0; 12]);
            central.extend_from_slice(&offset.to_le_bytes());
            central.extend_from_slice(name.as_bytes());
        }
        let cd_offset = out.len() as u32;
        let cd_len = central.len() as u32;
        out.extend_from_slice(&central);
        out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(&(files.len() as u16).to_le_bytes());
        out.extend_from_slice(&(files.len() as u16).to_le_bytes());
        out.extend_from_slice(&cd_len.to_le_bytes());
        out.extend_from_slice(&cd_offset.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out
    }

    #[test]
    fn comic_archive_becomes_one_page_per_picture() {
        let png = tiny_png();
        let zip = stored_zip(&[
            ("10.png", &png),
            ("2.png", &png),
            ("notes.txt", b"ignored"),
            ("__MACOSX/2.png", &png),
        ]);
        let pages = comic_pages(&zip).expect("read archive");
        assert_eq!(pages.len(), 2);

        let dir = tempfile::tempdir().expect("tempdir");
        let cbz = dir.path().join("book.cbz");
        std::fs::write(&cbz, &zip).expect("write cbz");
        let pdf = convert_to_pdf(&cbz).expect("convert");
        let doc = zpdf::PdfDocument::open(pdf).expect("valid pdf");
        assert_eq!(doc.page_count(), 2);
    }

    #[test]
    fn single_picture_becomes_a_pdf() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("pic.png");
        std::fs::write(&path, tiny_png()).expect("write png");
        let pdf = convert_to_pdf(&path).expect("convert");
        let doc = zpdf::PdfDocument::open(pdf).expect("valid pdf");
        assert_eq!(doc.page_count(), 1);
    }

    #[test]
    fn rar_comics_are_refused_clearly() {
        let err = comic_pages(b"Rar!\x1a\x07\x00 not a zip at all, padding padding").unwrap_err();
        assert!(err.contains("not a ZIP"), "{err}");
    }

    #[test]
    fn jpeg_header_is_parsed() {
        // SOI, SOF0 (len 17): precision 8, h=3, w=5, 3 components.
        let jpg = [
            0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x11, 0x08, 0x00, 0x03, 0x00, 0x05, 0x03, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0,
        ];
        assert_eq!(jpeg_info(&jpg), Some((5, 3, 3)));
    }
}
