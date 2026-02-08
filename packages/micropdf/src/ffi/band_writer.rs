//! C FFI for band-based output - MuPDF compatible
//! Safe Rust implementation of fz_band_writer

use super::{Handle, HandleStore};
use std::sync::LazyLock;

/// A wrapper for a raw pointer that implements Send + Sync.
/// SAFETY: The caller is responsible for ensuring the pointer
/// is only accessed from appropriate contexts.
#[derive(Debug, Clone, Copy)]
pub struct SendPtr(pub usize);

impl SendPtr {
    pub fn new(ptr: *mut std::ffi::c_void) -> Self {
        Self(ptr as usize)
    }

    pub fn as_ptr(&self) -> *mut std::ffi::c_void {
        self.0 as *mut std::ffi::c_void
    }

    pub fn is_null(&self) -> bool {
        self.0 == 0
    }
}

// SAFETY: We ensure the pointer is only used in the context of callbacks
// which are expected to be called from the same thread that set them up.
unsafe impl Send for SendPtr {}
unsafe impl Sync for SendPtr {}

/// Output format for band writer
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BandFormat {
    /// PNG format
    PNG = 0,
    /// JPEG format
    JPEG = 1,
    /// PNM (PPM/PGM/PBM) format
    PNM = 2,
    /// PAM format
    PAM = 3,
    /// TIFF format
    TIFF = 4,
    /// PWG (printer working group) raster
    PWG = 5,
    /// PCL format
    PCL = 6,
    /// PostScript format
    PS = 7,
    /// Raw pixel data
    Raw = 8,
}

/// Progress callback signature
pub type ProgressCallback =
    extern "C" fn(current: i32, total: i32, user_data: *mut std::ffi::c_void);

/// Band writer state
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BandWriterState {
    /// Writer is ready for header
    Ready = 0,
    /// Header has been written
    HeaderWritten = 1,
    /// Writing bands
    WritingBands = 2,
    /// All bands written, ready for trailer
    BandsComplete = 3,
    /// Trailer written, complete
    Complete = 4,
    /// Error state
    Error = 5,
}

/// Band writer configuration
#[derive(Debug, Clone)]
pub struct BandWriterConfig {
    /// Output format
    pub format: BandFormat,
    /// Image width
    pub width: i32,
    /// Image height
    pub height: i32,
    /// Number of color components
    pub n: i32,
    /// Alpha channel present
    pub alpha: bool,
    /// X resolution (DPI)
    pub x_res: i32,
    /// Y resolution (DPI)
    pub y_res: i32,
    /// JPEG quality (0-100)
    pub jpeg_quality: i32,
    /// Compression level (0-9 for PNG)
    pub compression: i32,
    /// Page number (for multi-page formats)
    pub page_num: i32,
    /// Total pages (for multi-page formats)
    pub total_pages: i32,
}

impl Default for BandWriterConfig {
    fn default() -> Self {
        Self {
            format: BandFormat::PNG,
            width: 0,
            height: 0,
            n: 3,
            alpha: false,
            x_res: 72,
            y_res: 72,
            jpeg_quality: 90,
            compression: 6,
            page_num: 1,
            total_pages: 1,
        }
    }
}

/// Band writer structure
#[derive(Debug)]
pub struct BandWriter {
    /// Configuration
    pub config: BandWriterConfig,
    /// Output stream handle
    pub output: Handle,
    /// Current state
    pub state: BandWriterState,
    /// Current band (row group) being written
    pub current_band: i32,
    /// Rows per band
    pub rows_per_band: i32,
    /// Total bands
    pub total_bands: i32,
    /// Bytes written so far
    pub bytes_written: usize,
    /// Progress callback
    pub progress_fn: Option<ProgressCallback>,
    /// Progress callback user data (wrapped for Send+Sync)
    pub progress_data: SendPtr,
    /// Accumulated output data
    pub output_buffer: Vec<u8>,
}

impl Default for BandWriter {
    fn default() -> Self {
        Self {
            config: BandWriterConfig::default(),
            output: 0,
            state: BandWriterState::Ready,
            current_band: 0,
            rows_per_band: 16,
            total_bands: 0,
            bytes_written: 0,
            progress_fn: None,
            progress_data: SendPtr::new(std::ptr::null_mut()),
            output_buffer: Vec::new(),
        }
    }
}

/// Global band writer storage
pub static BAND_WRITERS: LazyLock<HandleStore<BandWriter>> = LazyLock::new(HandleStore::new);

// ============================================================================
// Band Writer Creation
// ============================================================================

/// Create a new band writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_band_writer(_ctx: Handle, output: Handle, format: i32) -> Handle {
    let fmt = match format {
        1 => BandFormat::JPEG,
        2 => BandFormat::PNM,
        3 => BandFormat::PAM,
        4 => BandFormat::TIFF,
        5 => BandFormat::PWG,
        6 => BandFormat::PCL,
        7 => BandFormat::PS,
        8 => BandFormat::Raw,
        _ => BandFormat::PNG,
    };

    let writer = BandWriter {
        output,
        config: BandWriterConfig {
            format: fmt,
            ..Default::default()
        },
        ..Default::default()
    };

    BAND_WRITERS.insert(writer)
}

/// Create band writer with full configuration
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_band_writer_with_config(
    _ctx: Handle,
    output: Handle,
    format: i32,
    width: i32,
    height: i32,
    n: i32,
    alpha: i32,
) -> Handle {
    let fmt = match format {
        1 => BandFormat::JPEG,
        2 => BandFormat::PNM,
        3 => BandFormat::PAM,
        4 => BandFormat::TIFF,
        5 => BandFormat::PWG,
        6 => BandFormat::PCL,
        7 => BandFormat::PS,
        8 => BandFormat::Raw,
        _ => BandFormat::PNG,
    };

    let rows_per_band = 16;
    let total_bands = (height + rows_per_band - 1) / rows_per_band;

    let writer = BandWriter {
        output,
        config: BandWriterConfig {
            format: fmt,
            width,
            height,
            n,
            alpha: alpha != 0,
            ..Default::default()
        },
        rows_per_band,
        total_bands,
        ..Default::default()
    };

    BAND_WRITERS.insert(writer)
}

// ============================================================================
// Configuration
// ============================================================================

/// Set image dimensions
#[unsafe(no_mangle)]
pub extern "C" fn fz_band_writer_set_dimensions(
    _ctx: Handle,
    writer: Handle,
    width: i32,
    height: i32,
) {
    if let Some(w) = BAND_WRITERS.get(writer) {
        if let Ok(mut guard) = w.lock() {
            guard.config.width = width;
            guard.config.height = height;
            guard.total_bands = (height + guard.rows_per_band - 1) / guard.rows_per_band;
        }
    }
}

/// Set color components
#[unsafe(no_mangle)]
pub extern "C" fn fz_band_writer_set_components(_ctx: Handle, writer: Handle, n: i32, alpha: i32) {
    if let Some(w) = BAND_WRITERS.get(writer) {
        if let Ok(mut guard) = w.lock() {
            guard.config.n = n;
            guard.config.alpha = alpha != 0;
        }
    }
}

/// Set resolution
#[unsafe(no_mangle)]
pub extern "C" fn fz_band_writer_set_res(_ctx: Handle, writer: Handle, x_res: i32, y_res: i32) {
    if let Some(w) = BAND_WRITERS.get(writer) {
        if let Ok(mut guard) = w.lock() {
            guard.config.x_res = x_res.max(1);
            guard.config.y_res = y_res.max(1);
        }
    }
}

/// Set JPEG quality
#[unsafe(no_mangle)]
pub extern "C" fn fz_band_writer_set_jpeg_quality(_ctx: Handle, writer: Handle, quality: i32) {
    if let Some(w) = BAND_WRITERS.get(writer) {
        if let Ok(mut guard) = w.lock() {
            guard.config.jpeg_quality = quality.clamp(1, 100);
        }
    }
}

/// Set compression level
#[unsafe(no_mangle)]
pub extern "C" fn fz_band_writer_set_compression(_ctx: Handle, writer: Handle, level: i32) {
    if let Some(w) = BAND_WRITERS.get(writer) {
        if let Ok(mut guard) = w.lock() {
            guard.config.compression = level.clamp(0, 9);
        }
    }
}

/// Set rows per band
#[unsafe(no_mangle)]
pub extern "C" fn fz_band_writer_set_rows_per_band(_ctx: Handle, writer: Handle, rows: i32) {
    if let Some(w) = BAND_WRITERS.get(writer) {
        if let Ok(mut guard) = w.lock() {
            guard.rows_per_band = rows.max(1);
            if guard.config.height > 0 {
                guard.total_bands =
                    (guard.config.height + guard.rows_per_band - 1) / guard.rows_per_band;
            }
        }
    }
}

/// Set page info (for multi-page formats)
#[unsafe(no_mangle)]
pub extern "C" fn fz_band_writer_set_page_info(
    _ctx: Handle,
    writer: Handle,
    page_num: i32,
    total_pages: i32,
) {
    if let Some(w) = BAND_WRITERS.get(writer) {
        if let Ok(mut guard) = w.lock() {
            guard.config.page_num = page_num.max(1);
            guard.config.total_pages = total_pages.max(1);
        }
    }
}

/// Set progress callback
#[unsafe(no_mangle)]
pub extern "C" fn fz_band_writer_set_progress(
    _ctx: Handle,
    writer: Handle,
    callback: ProgressCallback,
    user_data: *mut std::ffi::c_void,
) {
    if let Some(w) = BAND_WRITERS.get(writer) {
        if let Ok(mut guard) = w.lock() {
            guard.progress_fn = Some(callback);
            guard.progress_data = SendPtr::new(user_data);
        }
    }
}

// ============================================================================
// Writing Operations
// ============================================================================

/// Write file header
#[unsafe(no_mangle)]
pub extern "C" fn fz_band_writer_write_header(_ctx: Handle, writer: Handle) -> i32 {
    if let Some(w) = BAND_WRITERS.get(writer) {
        if let Ok(mut guard) = w.lock() {
            if guard.state != BandWriterState::Ready {
                return 0;
            }

            // Generate header based on format
            let header = match guard.config.format {
                BandFormat::PNG => generate_png_header(&guard.config),
                BandFormat::PNM => generate_pnm_header(&guard.config),
                BandFormat::PAM => generate_pam_header(&guard.config),
                _ => Vec::new(),
            };

            guard.output_buffer.extend_from_slice(&header);
            guard.bytes_written += header.len();
            guard.state = BandWriterState::HeaderWritten;

            return 1;
        }
    }
    0
}

/// Write a band of pixel data
///
/// # Safety
/// `data` must point to valid pixel data of at least `band_rows * width * (n + alpha)` bytes.
#[unsafe(no_mangle)]
pub extern "C" fn fz_band_writer_write_band(
    _ctx: Handle,
    writer: Handle,
    band_rows: i32,
    data: *const u8,
) -> i32 {
    if data.is_null() || band_rows <= 0 {
        return 0;
    }

    if let Some(w) = BAND_WRITERS.get(writer) {
        if let Ok(mut guard) = w.lock() {
            if guard.state != BandWriterState::HeaderWritten
                && guard.state != BandWriterState::WritingBands
            {
                return 0;
            }

            guard.state = BandWriterState::WritingBands;

            let components = guard.config.n + if guard.config.alpha { 1 } else { 0 };
            let row_size = (guard.config.width * components) as usize;
            let band_size = row_size * band_rows as usize;

            let band_data = unsafe { std::slice::from_raw_parts(data, band_size) };

            // For raw format, just append data
            // For other formats, encoding would happen here
            guard.output_buffer.extend_from_slice(band_data);
            guard.bytes_written += band_size;
            guard.current_band += 1;

            // Call progress callback
            if let Some(callback) = guard.progress_fn {
                callback(
                    guard.current_band,
                    guard.total_bands,
                    guard.progress_data.as_ptr(),
                );
            }

            if guard.current_band >= guard.total_bands {
                guard.state = BandWriterState::BandsComplete;
            }

            return 1;
        }
    }
    0
}

/// Write file trailer/footer
#[unsafe(no_mangle)]
pub extern "C" fn fz_band_writer_write_trailer(_ctx: Handle, writer: Handle) -> i32 {
    if let Some(w) = BAND_WRITERS.get(writer) {
        if let Ok(mut guard) = w.lock() {
            if guard.state != BandWriterState::BandsComplete {
                return 0;
            }

            // Generate trailer based on format
            let trailer = match guard.config.format {
                BandFormat::PNG => generate_png_trailer(),
                _ => Vec::new(),
            };

            guard.output_buffer.extend_from_slice(&trailer);
            guard.bytes_written += trailer.len();
            guard.state = BandWriterState::Complete;

            return 1;
        }
    }
    0
}

// ============================================================================
// Query Functions
// ============================================================================

/// Get current state
#[unsafe(no_mangle)]
pub extern "C" fn fz_band_writer_state(_ctx: Handle, writer: Handle) -> i32 {
    if let Some(w) = BAND_WRITERS.get(writer) {
        if let Ok(guard) = w.lock() {
            return guard.state as i32;
        }
    }
    BandWriterState::Error as i32
}

/// Get current band number
#[unsafe(no_mangle)]
pub extern "C" fn fz_band_writer_current_band(_ctx: Handle, writer: Handle) -> i32 {
    if let Some(w) = BAND_WRITERS.get(writer) {
        if let Ok(guard) = w.lock() {
            return guard.current_band;
        }
    }
    0
}

/// Get total bands
#[unsafe(no_mangle)]
pub extern "C" fn fz_band_writer_total_bands(_ctx: Handle, writer: Handle) -> i32 {
    if let Some(w) = BAND_WRITERS.get(writer) {
        if let Ok(guard) = w.lock() {
            return guard.total_bands;
        }
    }
    0
}

/// Get bytes written
#[unsafe(no_mangle)]
pub extern "C" fn fz_band_writer_bytes_written(_ctx: Handle, writer: Handle) -> usize {
    if let Some(w) = BAND_WRITERS.get(writer) {
        if let Ok(guard) = w.lock() {
            return guard.bytes_written;
        }
    }
    0
}

/// Get progress (0.0 to 1.0)
#[unsafe(no_mangle)]
pub extern "C" fn fz_band_writer_progress(_ctx: Handle, writer: Handle) -> f32 {
    if let Some(w) = BAND_WRITERS.get(writer) {
        if let Ok(guard) = w.lock() {
            if guard.total_bands == 0 {
                return 0.0;
            }
            return guard.current_band as f32 / guard.total_bands as f32;
        }
    }
    0.0
}

/// Get output buffer (for in-memory output)
#[unsafe(no_mangle)]
pub extern "C" fn fz_band_writer_get_output(
    _ctx: Handle,
    writer: Handle,
    size: *mut usize,
) -> *const u8 {
    if let Some(w) = BAND_WRITERS.get(writer) {
        if let Ok(guard) = w.lock() {
            if !size.is_null() {
                unsafe { *size = guard.output_buffer.len() };
            }
            return guard.output_buffer.as_ptr();
        }
    }
    if !size.is_null() {
        unsafe { *size = 0 };
    }
    std::ptr::null()
}

// ============================================================================
// Helper Functions
// ============================================================================

fn generate_png_header(config: &BandWriterConfig) -> Vec<u8> {
    let mut header = Vec::new();

    // PNG signature
    header.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);

    // IHDR chunk (simplified)
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(config.width as u32).to_be_bytes());
    ihdr.extend_from_slice(&(config.height as u32).to_be_bytes());
    ihdr.push(8); // Bit depth

    // Color type: 0=gray, 2=RGB, 4=gray+alpha, 6=RGBA
    let color_type = match (config.n, config.alpha) {
        (1, false) => 0,
        (1, true) => 4,
        (3, false) => 2,
        (3, true) => 6,
        _ => 2,
    };
    ihdr.push(color_type);
    ihdr.push(0); // Compression method
    ihdr.push(0); // Filter method
    ihdr.push(0); // Interlace method

    // Write IHDR chunk
    write_png_chunk(&mut header, b"IHDR", &ihdr);

    header
}

fn generate_png_trailer() -> Vec<u8> {
    let mut trailer = Vec::new();
    write_png_chunk(&mut trailer, b"IEND", &[]);
    trailer
}

fn write_png_chunk(output: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
    output.extend_from_slice(&(data.len() as u32).to_be_bytes());
    output.extend_from_slice(chunk_type);
    output.extend_from_slice(data);

    // CRC (simplified - just use 0 for now)
    let crc = 0u32;
    output.extend_from_slice(&crc.to_be_bytes());
}

fn generate_pnm_header(config: &BandWriterConfig) -> Vec<u8> {
    let magic = if config.n == 1 { "P5" } else { "P6" };
    format!("{}\n{} {}\n255\n", magic, config.width, config.height).into_bytes()
}

fn generate_pam_header(config: &BandWriterConfig) -> Vec<u8> {
    let tupltype = match (config.n, config.alpha) {
        (1, false) => "GRAYSCALE",
        (1, true) => "GRAYSCALE_ALPHA",
        (3, false) => "RGB",
        (3, true) => "RGB_ALPHA",
        _ => "RGB",
    };

    let depth = config.n + if config.alpha { 1 } else { 0 };

    format!(
        "P7\nWIDTH {}\nHEIGHT {}\nDEPTH {}\nMAXVAL 255\nTUPLTYPE {}\nENDHDR\n",
        config.width, config.height, depth, tupltype
    )
    .into_bytes()
}

// ============================================================================
// Reference Counting
// ============================================================================

/// Keep band writer reference
#[unsafe(no_mangle)]
pub extern "C" fn fz_keep_band_writer(_ctx: Handle, writer: Handle) -> Handle {
    BAND_WRITERS.keep(writer)
}

/// Drop band writer reference
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_band_writer(_ctx: Handle, writer: Handle) {
    BAND_WRITERS.remove(writer);
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_band_writer() {
        let writer = fz_new_band_writer(0, 1, 0); // PNG
        assert!(writer > 0);

        assert_eq!(
            fz_band_writer_state(0, writer),
            BandWriterState::Ready as i32
        );

        fz_drop_band_writer(0, writer);
    }

    #[test]
    fn test_band_writer_config() {
        let writer = fz_new_band_writer_with_config(0, 1, 0, 100, 200, 3, 0);

        fz_band_writer_set_res(0, writer, 300, 300);
        fz_band_writer_set_rows_per_band(0, writer, 32);

        // 200 rows / 32 rows per band = 7 bands (rounded up)
        assert_eq!(fz_band_writer_total_bands(0, writer), 7);

        fz_drop_band_writer(0, writer);
    }

    #[test]
    fn test_write_sequence() {
        let writer = fz_new_band_writer_with_config(0, 1, 8, 10, 20, 3, 0); // Raw format
        fz_band_writer_set_rows_per_band(0, writer, 10);

        // Write header
        assert_eq!(fz_band_writer_write_header(0, writer), 1);
        assert_eq!(
            fz_band_writer_state(0, writer),
            BandWriterState::HeaderWritten as i32
        );

        // Write bands
        let band_data = vec![128u8; 10 * 10 * 3];
        assert_eq!(
            fz_band_writer_write_band(0, writer, 10, band_data.as_ptr()),
            1
        );
        assert_eq!(
            fz_band_writer_write_band(0, writer, 10, band_data.as_ptr()),
            1
        );

        assert_eq!(
            fz_band_writer_state(0, writer),
            BandWriterState::BandsComplete as i32
        );

        // Write trailer
        assert_eq!(fz_band_writer_write_trailer(0, writer), 1);
        assert_eq!(
            fz_band_writer_state(0, writer),
            BandWriterState::Complete as i32
        );

        // Check output
        assert!(fz_band_writer_bytes_written(0, writer) > 0);

        fz_drop_band_writer(0, writer);
    }

    #[test]
    fn test_progress() {
        let writer = fz_new_band_writer_with_config(0, 1, 8, 10, 40, 3, 0);
        fz_band_writer_set_rows_per_band(0, writer, 10);

        assert_eq!(fz_band_writer_progress(0, writer), 0.0);

        fz_band_writer_write_header(0, writer);

        let band_data = vec![0u8; 10 * 10 * 3];
        fz_band_writer_write_band(0, writer, 10, band_data.as_ptr());
        assert_eq!(fz_band_writer_progress(0, writer), 0.25); // 1/4

        fz_band_writer_write_band(0, writer, 10, band_data.as_ptr());
        assert_eq!(fz_band_writer_progress(0, writer), 0.5); // 2/4

        fz_drop_band_writer(0, writer);
    }
}
