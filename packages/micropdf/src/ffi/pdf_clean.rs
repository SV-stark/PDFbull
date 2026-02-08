//! PDF Clean/Optimization FFI Module
//!
//! Provides PDF optimization, cleaning, linearization, and page rearrangement.

use crate::ffi::Handle;
use std::ffi::{CStr, CString, c_char};
use std::ptr;

// ============================================================================
// Type Aliases
// ============================================================================

type ContextHandle = Handle;
type DocumentHandle = Handle;
type OutputHandle = Handle;

// ============================================================================
// Structure Options
// ============================================================================

/// Structure tree handling options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub enum CleanStructureOption {
    /// Remove the structure tree entirely (default)
    #[default]
    Drop = 0,
    /// Preserve the structure tree
    Keep = 1,
}

/// Vectorize options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub enum CleanVectorizeOption {
    /// Leave pages unchanged (default)
    #[default]
    No = 0,
    /// Vectorize each page (flatten Type 3 fonts)
    Yes = 1,
}

// ============================================================================
// Encryption Methods
// ============================================================================

/// Encryption method for PDF output
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub enum EncryptionMethod {
    /// Keep existing encryption
    #[default]
    Keep = 0,
    /// Remove encryption
    None = 1,
    /// RC4 40-bit encryption
    Rc4_40 = 2,
    /// RC4 128-bit encryption
    Rc4_128 = 3,
    /// AES 128-bit encryption
    Aes128 = 4,
    /// AES 256-bit encryption
    Aes256 = 5,
}

// ============================================================================
// Compression Methods
// ============================================================================

/// Compression method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub enum CompressionMethod {
    /// No compression
    #[default]
    None = 0,
    /// Zlib/Deflate compression
    Zlib = 1,
    /// Brotli compression
    Brotli = 2,
}

// ============================================================================
// Write Options
// ============================================================================

/// PDF write options
#[derive(Debug, Clone)]
#[repr(C)]
pub struct WriteOptions {
    /// Write just the changed objects (incremental save)
    pub do_incremental: i32,
    /// Pretty-print dictionaries and arrays
    pub do_pretty: i32,
    /// ASCII hex encode binary streams
    pub do_ascii: i32,
    /// Compress streams (0=none, 1=zlib, 2=brotli)
    pub do_compress: i32,
    /// Compress (or leave compressed) image streams
    pub do_compress_images: i32,
    /// Compress (or leave compressed) font streams
    pub do_compress_fonts: i32,
    /// Decompress streams (except images/fonts)
    pub do_decompress: i32,
    /// Garbage collect objects (1=gc, 2=renumber, 3=deduplicate)
    pub do_garbage: i32,
    /// Write linearized PDF
    pub do_linear: i32,
    /// Clean content streams
    pub do_clean: i32,
    /// Sanitize content streams
    pub do_sanitize: i32,
    /// (Re)create appearance streams
    pub do_appearance: i32,
    /// Encryption method
    pub do_encrypt: i32,
    /// Don't regenerate ID
    pub dont_regenerate_id: i32,
    /// Document permissions
    pub permissions: i32,
    /// Owner password (UTF-8)
    pub opwd_utf8: [u8; 128],
    /// User password (UTF-8)
    pub upwd_utf8: [u8; 128],
    /// Snapshot mode (internal use)
    pub do_snapshot: i32,
    /// Preserve metadata when cleaning
    pub do_preserve_metadata: i32,
    /// Use object streams if possible
    pub do_use_objstms: i32,
    /// Compression effort (0=default, 1=min, 100=max)
    pub compression_effort: i32,
    /// Add labels to objects
    pub do_labels: i32,
}

impl Default for WriteOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl WriteOptions {
    pub fn new() -> Self {
        Self {
            do_incremental: 0,
            do_pretty: 0,
            do_ascii: 0,
            do_compress: 1, // Default to zlib compression
            do_compress_images: 1,
            do_compress_fonts: 1,
            do_decompress: 0,
            do_garbage: 0,
            do_linear: 0,
            do_clean: 0,
            do_sanitize: 0,
            do_appearance: 0,
            do_encrypt: 0,
            dont_regenerate_id: 0,
            permissions: -1, // All permissions
            opwd_utf8: [0; 128],
            upwd_utf8: [0; 128],
            do_snapshot: 0,
            do_preserve_metadata: 0,
            do_use_objstms: 0,
            compression_effort: 0,
            do_labels: 0,
        }
    }

    /// Set owner password
    pub fn set_owner_password(&mut self, password: &str) {
        let bytes = password.as_bytes();
        let len = bytes.len().min(127);
        self.opwd_utf8[..len].copy_from_slice(&bytes[..len]);
        self.opwd_utf8[len] = 0;
    }

    /// Set user password
    pub fn set_user_password(&mut self, password: &str) {
        let bytes = password.as_bytes();
        let len = bytes.len().min(127);
        self.upwd_utf8[..len].copy_from_slice(&bytes[..len]);
        self.upwd_utf8[len] = 0;
    }

    /// Parse option string (matches mutool clean options)
    pub fn parse(&mut self, args: &str) {
        for c in args.chars() {
            match c {
                'g' => self.do_garbage = 1,
                'G' => self.do_garbage = 2,
                'D' => self.do_garbage = 3,
                'd' => self.do_decompress = 1,
                'i' => {
                    self.do_decompress = 1;
                    self.do_compress_images = 0;
                }
                'f' => {
                    self.do_decompress = 1;
                    self.do_compress_fonts = 0;
                }
                'l' => self.do_linear = 1,
                'a' => self.do_ascii = 1,
                'z' => self.do_compress = 1,
                'Z' => self.do_compress = 2, // Brotli
                'c' => self.do_clean = 1,
                's' => self.do_sanitize = 1,
                'p' => self.do_pretty = 1,
                'A' => self.do_appearance = 1,
                'm' => self.do_preserve_metadata = 1,
                'o' => self.do_use_objstms = 1,
                'L' => self.do_labels = 1,
                _ => {}
            }
        }
    }

    /// Format options to string
    pub fn format(&self) -> String {
        let mut s = String::new();
        if self.do_garbage == 1 {
            s.push('g');
        }
        if self.do_garbage == 2 {
            s.push('G');
        }
        if self.do_garbage == 3 {
            s.push('D');
        }
        if self.do_decompress != 0 {
            s.push('d');
        }
        if self.do_linear != 0 {
            s.push('l');
        }
        if self.do_ascii != 0 {
            s.push('a');
        }
        if self.do_compress == 1 {
            s.push('z');
        }
        if self.do_compress == 2 {
            s.push('Z');
        }
        if self.do_clean != 0 {
            s.push('c');
        }
        if self.do_sanitize != 0 {
            s.push('s');
        }
        if self.do_pretty != 0 {
            s.push('p');
        }
        if self.do_appearance != 0 {
            s.push('A');
        }
        if self.do_preserve_metadata != 0 {
            s.push('m');
        }
        if self.do_use_objstms != 0 {
            s.push('o');
        }
        if self.do_labels != 0 {
            s.push('L');
        }
        s
    }
}

// ============================================================================
// Image Rewriter Options
// ============================================================================

/// Image rewriter options
#[derive(Debug, Clone, Default)]
#[repr(C)]
pub struct ImageRewriterOptions {
    /// Target color depth (0 = keep)
    pub color_depth: i32,
    /// Target DPI (0 = keep)
    pub dpi: i32,
    /// JPEG quality (0-100)
    pub jpeg_quality: i32,
    /// Recompress images
    pub recompress: i32,
}

// ============================================================================
// Clean Options
// ============================================================================

/// PDF clean options
#[derive(Debug, Clone)]
#[repr(C)]
pub struct CleanOptions {
    /// Write options
    pub write: WriteOptions,
    /// Image rewriter options
    pub image: ImageRewriterOptions,
    /// Subset fonts
    pub subset_fonts: i32,
    /// Structure tree handling
    pub structure: CleanStructureOption,
    /// Vectorize option
    pub vectorize: CleanVectorizeOption,
}

impl Default for CleanOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl CleanOptions {
    pub fn new() -> Self {
        Self {
            write: WriteOptions::new(),
            image: ImageRewriterOptions::default(),
            subset_fonts: 0,
            structure: CleanStructureOption::Drop,
            vectorize: CleanVectorizeOption::No,
        }
    }

    /// Create options for optimization
    pub fn optimize() -> Self {
        let mut opts = Self::new();
        opts.write.do_garbage = 3; // Deduplicate
        opts.write.do_compress = 1;
        opts.write.do_clean = 1;
        opts.write.do_sanitize = 1;
        opts.subset_fonts = 1;
        opts
    }

    /// Create options for linearization
    pub fn linearize() -> Self {
        let mut opts = Self::new();
        opts.write.do_linear = 1;
        opts.write.do_garbage = 1;
        opts.write.do_compress = 1;
        opts
    }
}

// ============================================================================
// FFI Functions - Default Options
// ============================================================================

/// Get default write options.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_default_write_options() -> WriteOptions {
    WriteOptions::new()
}

/// Get default clean options.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_default_clean_options() -> CleanOptions {
    CleanOptions::new()
}

// ============================================================================
// FFI Functions - Parse Options
// ============================================================================

/// Parse write options from string.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_parse_write_options(
    _ctx: ContextHandle,
    opts: *mut WriteOptions,
    args: *const c_char,
) -> *mut WriteOptions {
    if opts.is_null() || args.is_null() {
        return opts;
    }

    let args_str = unsafe { CStr::from_ptr(args).to_str().unwrap_or("") };
    unsafe {
        (*opts).parse(args_str);
    }
    opts
}

/// Format write options to string.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_format_write_options(
    _ctx: ContextHandle,
    buffer: *mut c_char,
    buffer_len: usize,
    opts: *const WriteOptions,
) -> *mut c_char {
    if buffer.is_null() || buffer_len == 0 || opts.is_null() {
        return buffer;
    }

    let formatted = unsafe { (*opts).format() };
    let len = formatted.len().min(buffer_len - 1);
    unsafe {
        ptr::copy_nonoverlapping(formatted.as_ptr(), buffer as *mut u8, len);
        *buffer.add(len) = 0;
    }
    buffer
}

// ============================================================================
// FFI Functions - Document Operations
// ============================================================================

/// Check if document can be saved incrementally.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_can_be_saved_incrementally(_ctx: ContextHandle, _doc: DocumentHandle) -> i32 {
    // In a full implementation, this would check document state
    1
}

/// Check if document has unsaved signatures.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_has_unsaved_sigs(_ctx: ContextHandle, _doc: DocumentHandle) -> i32 {
    // In a full implementation, this would check for unsigned signature fields
    0
}

/// Save document to file.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_save_document(
    _ctx: ContextHandle,
    doc: DocumentHandle,
    filename: *const c_char,
    _opts: *const WriteOptions,
) {
    if filename.is_null() {
        return;
    }

    // Get document and write its data to file
    if let Some(document) = super::DOCUMENTS.get(doc) {
        if let Ok(guard) = document.lock() {
            // SAFETY: Caller guarantees filename is a valid null-terminated C string
            let c_str = unsafe { std::ffi::CStr::from_ptr(filename) };
            if let Ok(path) = c_str.to_str() {
                let _ = std::fs::write(path, guard.data());
            }
        }
    }
}

/// Write document to output stream.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_write_document(
    _ctx: ContextHandle,
    _doc: DocumentHandle,
    _out: OutputHandle,
    _opts: *const WriteOptions,
) {
    // In a full implementation, this would write to the output stream
}

/// Save document snapshot.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_save_snapshot(
    _ctx: ContextHandle,
    _doc: DocumentHandle,
    filename: *const c_char,
) {
    if filename.is_null() {
        return;
    }
    // In a full implementation, this would save a snapshot
}

/// Write document snapshot to output stream.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_write_snapshot(
    _ctx: ContextHandle,
    _doc: DocumentHandle,
    _out: OutputHandle,
) {
    // In a full implementation, this would write a snapshot
}

/// Save document journal.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_save_journal(
    _ctx: ContextHandle,
    _doc: DocumentHandle,
    filename: *const c_char,
) {
    if filename.is_null() {
        return;
    }
    // In a full implementation, this would save the journal
}

/// Write document journal to output stream.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_write_journal(_ctx: ContextHandle, _doc: DocumentHandle, _out: OutputHandle) {
    // In a full implementation, this would write the journal
}

// ============================================================================
// FFI Functions - Clean Operations
// ============================================================================

/// Clean a PDF file.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_clean_file(
    _ctx: ContextHandle,
    infile: *const c_char,
    outfile: *const c_char,
    _password: *const c_char,
    _opts: *const CleanOptions,
    _retainlen: i32,
    _retainlist: *const *const c_char,
) {
    if infile.is_null() || outfile.is_null() {
        return;
    }
    // In a full implementation, this would clean the PDF
}

/// Rearrange pages in document.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_rearrange_pages(
    _ctx: ContextHandle,
    _doc: DocumentHandle,
    count: i32,
    pages: *const i32,
    _structure: CleanStructureOption,
) {
    if count <= 0 || pages.is_null() {
        return;
    }
    // In a full implementation, this would rearrange pages
}

/// Vectorize pages in document.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_vectorize_pages(
    _ctx: ContextHandle,
    _doc: DocumentHandle,
    _count: i32,
    _pages: *const i32,
    _vectorize: CleanVectorizeOption,
) {
    // In a full implementation, this would vectorize pages
}

// ============================================================================
// FFI Functions - Object Operations
// ============================================================================

/// Clean a PDF object (remove unused entries).
#[unsafe(no_mangle)]
pub extern "C" fn pdf_clean_object_entries(_ctx: ContextHandle, _obj: Handle) {
    // In a full implementation, this would clean the object
}

// ============================================================================
// FFI Functions - Optimization Helpers
// ============================================================================

/// Optimize PDF (convenience function).
#[unsafe(no_mangle)]
pub extern "C" fn pdf_optimize(ctx: ContextHandle, doc: DocumentHandle, filename: *const c_char) {
    let opts = CleanOptions::optimize();
    pdf_save_document(ctx, doc, filename, &opts.write);
}

/// Linearize PDF (convenience function).
#[unsafe(no_mangle)]
pub extern "C" fn pdf_linearize(ctx: ContextHandle, doc: DocumentHandle, filename: *const c_char) {
    let opts = CleanOptions::linearize();
    pdf_save_document(ctx, doc, filename, &opts.write);
}

/// Compress all streams in document.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_compress_streams(_ctx: ContextHandle, _doc: DocumentHandle, method: i32) {
    let _compression = match method {
        1 => CompressionMethod::Zlib,
        2 => CompressionMethod::Brotli,
        _ => CompressionMethod::None,
    };
    // In a full implementation, this would compress streams
}

/// Decompress all streams in document.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_decompress_streams(_ctx: ContextHandle, _doc: DocumentHandle) {
    // In a full implementation, this would decompress streams
}

/// Create object streams.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_create_object_streams(_ctx: ContextHandle, _doc: DocumentHandle) {
    // In a full implementation, this would create object streams
}

/// Remove object streams.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_remove_object_streams(_ctx: ContextHandle, _doc: DocumentHandle) {
    // In a full implementation, this would remove object streams
}

/// Garbage collect unused objects.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_garbage_collect(_ctx: ContextHandle, _doc: DocumentHandle, level: i32) {
    let _gc_level = match level {
        1 => "collect",
        2 => "renumber",
        3 => "deduplicate",
        _ => "none",
    };
    // In a full implementation, this would garbage collect
}

/// Deduplicate objects.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_deduplicate_objects(_ctx: ContextHandle, _doc: DocumentHandle) {
    // In a full implementation, this would deduplicate objects
}

/// Renumber objects.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_renumber_objects(_ctx: ContextHandle, _doc: DocumentHandle) {
    // In a full implementation, this would renumber objects
}

/// Remove unused resources.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_remove_unused_resources(_ctx: ContextHandle, _doc: DocumentHandle) {
    // In a full implementation, this would remove unused resources
}

// ============================================================================
// FFI Functions - Encryption
// ============================================================================

/// Set document encryption.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_encryption(
    _ctx: ContextHandle,
    opts: *mut WriteOptions,
    method: i32,
    permissions: i32,
    owner_pwd: *const c_char,
    user_pwd: *const c_char,
) {
    if opts.is_null() {
        return;
    }

    unsafe {
        (*opts).do_encrypt = method;
        (*opts).permissions = permissions;

        if !owner_pwd.is_null() {
            if let Ok(pwd) = CStr::from_ptr(owner_pwd).to_str() {
                (*opts).set_owner_password(pwd);
            }
        }

        if !user_pwd.is_null() {
            if let Ok(pwd) = CStr::from_ptr(user_pwd).to_str() {
                (*opts).set_user_password(pwd);
            }
        }
    }
}

/// Remove document encryption.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_remove_encryption(_ctx: ContextHandle, opts: *mut WriteOptions) {
    if opts.is_null() {
        return;
    }
    unsafe {
        (*opts).do_encrypt = EncryptionMethod::None as i32;
    }
}

// ============================================================================
// FFI Functions - Free Strings
// ============================================================================

/// Free a string allocated by clean functions.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_clean_free_string(_ctx: ContextHandle, s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_options_default() {
        let opts = WriteOptions::new();
        assert_eq!(opts.do_incremental, 0);
        assert_eq!(opts.do_compress, 1);
        assert_eq!(opts.do_garbage, 0);
        assert_eq!(opts.do_linear, 0);
    }

    #[test]
    fn test_write_options_parse() {
        let mut opts = WriteOptions::new();
        opts.parse("glzcs");
        assert_eq!(opts.do_garbage, 1);
        assert_eq!(opts.do_linear, 1);
        assert_eq!(opts.do_compress, 1);
        assert_eq!(opts.do_clean, 1);
        assert_eq!(opts.do_sanitize, 1);
    }

    #[test]
    fn test_write_options_format() {
        let mut opts = WriteOptions::new();
        opts.do_garbage = 1;
        opts.do_linear = 1;
        opts.do_compress = 1;
        let formatted = opts.format();
        assert!(formatted.contains('g'));
        assert!(formatted.contains('l'));
        assert!(formatted.contains('z'));
    }

    #[test]
    fn test_write_options_password() {
        let mut opts = WriteOptions::new();
        opts.set_owner_password("owner123");
        opts.set_user_password("user456");

        let owner = std::str::from_utf8(&opts.opwd_utf8[..8]).unwrap();
        assert_eq!(owner, "owner123");

        let user = std::str::from_utf8(&opts.upwd_utf8[..7]).unwrap();
        assert_eq!(user, "user456");
    }

    #[test]
    fn test_clean_options_default() {
        let opts = CleanOptions::new();
        assert_eq!(opts.subset_fonts, 0);
        assert_eq!(opts.structure, CleanStructureOption::Drop);
        assert_eq!(opts.vectorize, CleanVectorizeOption::No);
    }

    #[test]
    fn test_clean_options_optimize() {
        let opts = CleanOptions::optimize();
        assert_eq!(opts.write.do_garbage, 3);
        assert_eq!(opts.write.do_compress, 1);
        assert_eq!(opts.write.do_clean, 1);
        assert_eq!(opts.subset_fonts, 1);
    }

    #[test]
    fn test_clean_options_linearize() {
        let opts = CleanOptions::linearize();
        assert_eq!(opts.write.do_linear, 1);
        assert_eq!(opts.write.do_garbage, 1);
    }

    #[test]
    fn test_structure_option() {
        assert_eq!(CleanStructureOption::Drop as i32, 0);
        assert_eq!(CleanStructureOption::Keep as i32, 1);
    }

    #[test]
    fn test_vectorize_option() {
        assert_eq!(CleanVectorizeOption::No as i32, 0);
        assert_eq!(CleanVectorizeOption::Yes as i32, 1);
    }

    #[test]
    fn test_encryption_method() {
        assert_eq!(EncryptionMethod::Keep as i32, 0);
        assert_eq!(EncryptionMethod::None as i32, 1);
        assert_eq!(EncryptionMethod::Aes256 as i32, 5);
    }

    #[test]
    fn test_ffi_default_options() {
        let write_opts = pdf_default_write_options();
        assert_eq!(write_opts.do_compress, 1);

        let clean_opts = pdf_default_clean_options();
        assert_eq!(clean_opts.structure, CleanStructureOption::Drop);
    }

    #[test]
    fn test_ffi_parse_options() {
        let mut opts = WriteOptions::new();
        let args = CString::new("glzcs").unwrap();
        pdf_parse_write_options(0, &mut opts, args.as_ptr());
        assert_eq!(opts.do_garbage, 1);
        assert_eq!(opts.do_linear, 1);
    }

    #[test]
    fn test_ffi_format_options() {
        let mut opts = WriteOptions::new();
        opts.do_garbage = 1;
        opts.do_linear = 1;

        let mut buffer = [0u8; 64];
        pdf_format_write_options(0, buffer.as_mut_ptr() as *mut c_char, 64, &opts);

        let result = unsafe { CStr::from_ptr(buffer.as_ptr() as *const c_char) };
        let s = result.to_str().unwrap();
        assert!(s.contains('g'));
        assert!(s.contains('l'));
    }

    #[test]
    fn test_ffi_can_save_incrementally() {
        let result = pdf_can_be_saved_incrementally(0, 0);
        assert_eq!(result, 1);
    }

    #[test]
    fn test_ffi_has_unsaved_sigs() {
        let result = pdf_has_unsaved_sigs(0, 0);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_ffi_set_encryption() {
        let mut opts = WriteOptions::new();
        let owner = CString::new("owner").unwrap();
        let user = CString::new("user").unwrap();

        pdf_set_encryption(0, &mut opts, 5, 0xFFFF, owner.as_ptr(), user.as_ptr());

        assert_eq!(opts.do_encrypt, 5); // AES-256
        assert_eq!(opts.permissions, 0xFFFF);
    }

    #[test]
    fn test_ffi_remove_encryption() {
        let mut opts = WriteOptions::new();
        opts.do_encrypt = 5;

        pdf_remove_encryption(0, &mut opts);
        assert_eq!(opts.do_encrypt, 1); // None
    }
}
