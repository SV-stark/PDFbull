//! C FFI Module - MuPDF API Compatible Exports
//!
//! This module provides C-compatible exports that match MuPDF's API.
//! Uses safe Rust patterns with handle-based resource management.

// Clippy false positive: FFI functions with #[unsafe(no_mangle)] are inherently unsafe
// and all pointer dereferences are wrapped in unsafe blocks after null checks
#![allow(clippy::not_unsafe_ptr_arg_deref)]

pub mod annot;
pub mod archive;
pub mod band_writer;
pub mod barcode;
pub mod bidi;
pub mod bitmap;
pub mod buffer;
pub mod buffered_io;
pub mod cbz;
pub mod color;
pub mod colorspace;
pub mod compat;
pub mod compress;
pub mod context;
pub mod convenience;
pub mod cookie;
pub mod data_locality;
pub mod deskew;
pub mod device;
pub mod display_list;
pub mod document;
pub mod draw_device;
pub mod enhanced;
pub mod epub;
pub mod ffi_safety;
pub mod filter;
pub mod font;
pub mod form;
pub mod geometry;
pub mod glyph;
pub mod glyph_cache;
pub mod gpu;
pub mod hashmap_util;
pub mod heap;
pub mod hints;
pub mod hyphen;
pub mod image;
pub mod json;
pub mod link;
pub mod lockfree;
pub mod log;
pub mod memory_profiler;
pub mod mmap;
pub mod ocr;
pub mod office;
pub mod outline;
pub mod output;
pub mod path;
pub mod pdf_3d;
pub mod pdf_clean;
pub mod pdf_cmap;
pub mod pdf_conformance;
pub mod pdf_event;
pub mod pdf_font;
pub mod pdf_image_rewriter;
pub mod pdf_interpret;
pub mod pdf_javascript;
pub mod pdf_layer;
pub mod pdf_name_table;
pub mod pdf_object;
pub mod pdf_page;
pub mod pdf_parse;
pub mod pdf_portfolio;
pub mod pdf_recolor;
pub mod pdf_redact;
pub mod pdf_resource;
pub mod pdf_signature;
pub mod pdf_xref;
pub mod pdf_zugferd;
pub mod pixmap;
pub mod pool;
pub mod separation;
pub mod shade;
pub mod simd_util;
pub mod stext;
pub mod store;
pub mod story;
pub mod stream;
pub mod string_util;
pub mod struct_layout;
pub mod svg;
pub mod table_detect;
pub mod text;
pub mod tile_render;
pub mod transition;
pub mod tree;
pub mod util;
pub mod write_pixmap;
pub mod writer;
pub mod xml;
pub mod xps;

// Safe helper functions for common FFI patterns
mod safe_helpers;

use std::collections::HashMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

/// Global handle manager for safe FFI resource management
static HANDLE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Type alias for handles
pub type Handle = u64;

/// Generate a new unique handle
pub fn new_handle() -> Handle {
    HANDLE_COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// Statistics for handle store tracking
#[derive(Debug, Default, Clone, Copy)]
pub struct HandleStoreStats {
    /// Total handles ever created
    pub total_created: u64,
    /// Total handles destroyed
    pub total_destroyed: u64,
    /// Current live handles
    pub current_count: u64,
    /// Peak concurrent handles
    pub peak_count: u64,
}

/// Thread-safe handle storage for a specific type
pub struct HandleStore<T> {
    store: Mutex<HashMap<Handle, Arc<Mutex<T>>>>,
    stats: Mutex<HandleStoreStats>,
}

impl<T> HandleStore<T> {
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
            stats: Mutex::new(HandleStoreStats::default()),
        }
    }

    /// Insert a value and return its handle.
    ///
    /// Returns a non-zero handle on success. The caller is responsible
    /// for eventually calling `remove()` to release the resource.
    #[must_use = "handle must be stored and later passed to remove() to avoid resource leaks"]
    pub fn insert(&self, value: T) -> Handle {
        let handle = new_handle();
        let mut store = self.store.lock().unwrap();
        store.insert(handle, Arc::new(Mutex::new(value)));

        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.total_created += 1;
            stats.current_count += 1;
            if stats.current_count > stats.peak_count {
                stats.peak_count = stats.current_count;
            }
        }

        handle
    }

    /// Get a reference to the value associated with a handle.
    pub fn get(&self, handle: Handle) -> Option<Arc<Mutex<T>>> {
        let store = self.store.lock().unwrap();
        store.get(&handle).cloned()
    }

    /// Remove a handle and return the value if it exists.
    pub fn remove(&self, handle: Handle) -> Option<Arc<Mutex<T>>> {
        let mut store = self.store.lock().unwrap();
        let result = store.remove(&handle);

        // Update stats
        if result.is_some() {
            if let Ok(mut stats) = self.stats.lock() {
                stats.total_destroyed += 1;
                stats.current_count = stats.current_count.saturating_sub(1);
            }
        }

        result
    }

    /// Keep (retain) a handle - returns the same handle.
    /// For reference counting, the Arc inside handles ref counting automatically.
    #[must_use = "returned handle should be used or the keep call is unnecessary"]
    pub fn keep(&self, handle: Handle) -> Handle {
        handle
    }

    /// Get current statistics for this handle store.
    pub fn stats(&self) -> HandleStoreStats {
        self.stats.lock().map(|s| *s).unwrap_or_default()
    }

    /// Get current number of live handles.
    pub fn len(&self) -> usize {
        self.store.lock().map(|s| s.len()).unwrap_or(0)
    }

    /// Check if store is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Check for potential leaks (handles that haven't been removed).
    /// Returns a list of handles that are still alive.
    pub fn get_live_handles(&self) -> Vec<Handle> {
        self.store
            .lock()
            .map(|s| s.keys().copied().collect())
            .unwrap_or_default()
    }
}

impl<T> Default for HandleStore<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for HandleStore<T> {
    fn drop(&mut self) {
        // Log if there are unreleased handles (potential leak detection)
        if let Ok(store) = self.store.lock() {
            let count = store.len();
            if count > 0 {
                // In debug builds, this helps identify leaks
                #[cfg(debug_assertions)]
                eprintln!(
                    "[HandleStore] Warning: {} unreleased handles at drop time",
                    count
                );
            }
        }
    }
}

// Lazy initialization for handle stores
use std::sync::LazyLock;

pub static CONTEXTS: LazyLock<HandleStore<context::Context>> = LazyLock::new(HandleStore::new);
pub static BUFFERS: LazyLock<HandleStore<buffer::Buffer>> = LazyLock::new(HandleStore::new);
pub static STREAMS: LazyLock<HandleStore<stream::Stream>> = LazyLock::new(HandleStore::new);
pub static PIXMAPS: LazyLock<HandleStore<pixmap::Pixmap>> = LazyLock::new(HandleStore::new);
pub static DOCUMENTS: LazyLock<HandleStore<document::Document>> = LazyLock::new(HandleStore::new);
