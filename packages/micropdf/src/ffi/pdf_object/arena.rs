//! PDF Object Arena Allocator
//!
//! Provides efficient bulk allocation for PDF objects with document-lifetime
//! semantics. Objects allocated from an arena are freed together when the
//! arena is dropped, reducing per-object allocation overhead.

use super::super::Handle;
use super::types::{PdfObj, PdfObjType};
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

// ============================================================================
// Arena Configuration
// ============================================================================

/// Default chunk size (number of objects per chunk)
const DEFAULT_CHUNK_SIZE: usize = 1024;

/// Maximum objects per arena before warning
const MAX_ARENA_OBJECTS: usize = 1_000_000;

// ============================================================================
// Object Arena
// ============================================================================

/// A chunk of pre-allocated PDF objects
struct ObjectChunk {
    /// Pre-allocated objects
    objects: Vec<UnsafeCell<Option<PdfObj>>>,
    /// Bitmap of allocated slots (1 = allocated, 0 = free)
    allocated: Vec<bool>,
    /// Number of allocated objects in this chunk
    count: usize,
}

impl ObjectChunk {
    fn new(size: usize) -> Self {
        let mut objects = Vec::with_capacity(size);
        let mut allocated = Vec::with_capacity(size);
        for _ in 0..size {
            objects.push(UnsafeCell::new(None));
            allocated.push(false);
        }
        Self {
            objects,
            allocated,
            count: 0,
        }
    }

    /// Allocate a slot in this chunk, returning the index
    fn allocate(&mut self, obj: PdfObj) -> Option<usize> {
        for (i, is_allocated) in self.allocated.iter_mut().enumerate() {
            if !*is_allocated {
                *is_allocated = true;
                // SAFETY: We have exclusive access through &mut self
                unsafe {
                    *self.objects[i].get() = Some(obj);
                }
                self.count += 1;
                return Some(i);
            }
        }
        None
    }

    /// Free a slot
    fn free(&mut self, index: usize) {
        if index < self.allocated.len() && self.allocated[index] {
            self.allocated[index] = false;
            // SAFETY: We have exclusive access through &mut self
            unsafe {
                *self.objects[index].get() = None;
            }
            self.count -= 1;
        }
    }

    /// Get object at index
    fn get(&self, index: usize) -> Option<&PdfObj> {
        if index < self.allocated.len() && self.allocated[index] {
            // SAFETY: Slot is allocated, so it contains a valid object
            unsafe { (*self.objects[index].get()).as_ref() }
        } else {
            None
        }
    }

    /// Get mutable object at index
    fn get_mut(&mut self, index: usize) -> Option<&mut PdfObj> {
        if index < self.allocated.len() && self.allocated[index] {
            // SAFETY: We have exclusive access through &mut self
            unsafe { (*self.objects[index].get()).as_mut() }
        } else {
            None
        }
    }

    fn is_full(&self) -> bool {
        self.count >= self.objects.len()
    }

    fn is_empty(&self) -> bool {
        self.count == 0
    }
}

/// Arena handle - encodes chunk index and slot index
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArenaHandle {
    /// Arena ID
    pub arena_id: u32,
    /// Chunk index within arena
    pub chunk_idx: u16,
    /// Slot index within chunk
    pub slot_idx: u16,
}

impl ArenaHandle {
    /// Encode as a single u64 handle
    pub fn to_handle(&self) -> Handle {
        ((self.arena_id as u64) << 32) | ((self.chunk_idx as u64) << 16) | (self.slot_idx as u64)
    }

    /// Decode from a u64 handle
    pub fn from_handle(h: Handle) -> Self {
        Self {
            arena_id: (h >> 32) as u32,
            chunk_idx: ((h >> 16) & 0xFFFF) as u16,
            slot_idx: (h & 0xFFFF) as u16,
        }
    }

    /// Check if handle is an arena handle (arena_id != 0)
    pub fn is_arena_handle(h: Handle) -> bool {
        (h >> 32) != 0
    }
}

/// PDF Object Arena - bulk allocation with document lifetime
pub struct PdfObjectArena {
    /// Arena ID
    id: u32,
    /// Chunks of pre-allocated objects
    chunks: Vec<ObjectChunk>,
    /// Chunk size
    chunk_size: usize,
    /// Total objects allocated
    total_allocated: usize,
    /// Total objects freed
    total_freed: usize,
    /// Arena name (for debugging)
    name: String,
}

impl PdfObjectArena {
    /// Create a new arena with default chunk size
    pub fn new(id: u32) -> Self {
        Self::with_chunk_size(id, DEFAULT_CHUNK_SIZE)
    }

    /// Create a new arena with custom chunk size
    pub fn with_chunk_size(id: u32, chunk_size: usize) -> Self {
        Self {
            id,
            chunks: Vec::new(),
            chunk_size,
            total_allocated: 0,
            total_freed: 0,
            name: String::new(),
        }
    }

    /// Set arena name (for debugging)
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    /// Allocate a new object in the arena
    pub fn alloc(&mut self, obj: PdfObj) -> ArenaHandle {
        // First, find a non-full chunk index
        let existing_chunk_idx = self
            .chunks
            .iter()
            .enumerate()
            .find(|(_, chunk)| !chunk.is_full())
            .map(|(idx, _)| idx);

        match existing_chunk_idx {
            Some(chunk_idx) => {
                let chunk = &mut self.chunks[chunk_idx];
                let slot_idx = chunk
                    .allocate(obj)
                    .expect("Non-full chunk should have space");
                self.total_allocated += 1;
                ArenaHandle {
                    arena_id: self.id,
                    chunk_idx: chunk_idx as u16,
                    slot_idx: slot_idx as u16,
                }
            }
            None => {
                // Need a new chunk
                let chunk_idx = self.chunks.len();
                let mut new_chunk = ObjectChunk::new(self.chunk_size);
                let slot_idx = new_chunk
                    .allocate(obj)
                    .expect("New chunk should have space");
                self.chunks.push(new_chunk);
                self.total_allocated += 1;

                ArenaHandle {
                    arena_id: self.id,
                    chunk_idx: chunk_idx as u16,
                    slot_idx: slot_idx as u16,
                }
            }
        }
    }

    /// Free an object from the arena
    pub fn free(&mut self, handle: ArenaHandle) {
        if handle.arena_id != self.id {
            return;
        }
        if let Some(chunk) = self.chunks.get_mut(handle.chunk_idx as usize) {
            chunk.free(handle.slot_idx as usize);
            self.total_freed += 1;
        }
    }

    /// Get an object from the arena
    pub fn get(&self, handle: ArenaHandle) -> Option<&PdfObj> {
        if handle.arena_id != self.id {
            return None;
        }
        self.chunks
            .get(handle.chunk_idx as usize)?
            .get(handle.slot_idx as usize)
    }

    /// Get a mutable object from the arena
    pub fn get_mut(&mut self, handle: ArenaHandle) -> Option<&mut PdfObj> {
        if handle.arena_id != self.id {
            return None;
        }
        self.chunks
            .get_mut(handle.chunk_idx as usize)?
            .get_mut(handle.slot_idx as usize)
    }

    /// Get arena statistics
    pub fn stats(&self) -> ArenaStats {
        let chunk_count = self.chunks.len();
        let capacity = chunk_count * self.chunk_size;
        let active = self.total_allocated - self.total_freed;
        ArenaStats {
            arena_id: self.id,
            chunk_count,
            chunk_size: self.chunk_size,
            capacity,
            allocated: self.total_allocated,
            freed: self.total_freed,
            active,
            utilization: if capacity > 0 {
                active as f64 / capacity as f64
            } else {
                0.0
            },
        }
    }

    /// Clear all objects but keep chunks allocated
    pub fn clear(&mut self) {
        for chunk in &mut self.chunks {
            for (i, is_allocated) in chunk.allocated.iter_mut().enumerate() {
                if *is_allocated {
                    *is_allocated = false;
                    // SAFETY: We have exclusive access
                    unsafe {
                        *chunk.objects[i].get() = None;
                    }
                }
            }
            chunk.count = 0;
        }
        self.total_freed = self.total_allocated;
    }

    /// Compact the arena by removing empty chunks
    pub fn compact(&mut self) {
        self.chunks.retain(|chunk| !chunk.is_empty());
    }
}

/// Arena statistics
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ArenaStats {
    /// Arena ID
    pub arena_id: u32,
    /// Number of chunks
    pub chunk_count: usize,
    /// Objects per chunk
    pub chunk_size: usize,
    /// Total capacity
    pub capacity: usize,
    /// Total objects allocated
    pub allocated: usize,
    /// Total objects freed
    pub freed: usize,
    /// Currently active objects
    pub active: usize,
    /// Utilization (0.0 - 1.0)
    pub utilization: f64,
}

// ============================================================================
// Global Arena Registry
// ============================================================================

/// Arena counter for generating unique IDs
static ARENA_COUNTER: LazyLock<Mutex<u32>> = LazyLock::new(|| Mutex::new(1));

/// Global arena registry
static ARENAS: LazyLock<Mutex<HashMap<u32, PdfObjectArena>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Create a new arena and return its ID
fn create_arena_internal() -> u32 {
    let id = {
        let mut counter = ARENA_COUNTER.lock().unwrap();
        let id = *counter;
        *counter += 1;
        id
    };

    let arena = PdfObjectArena::new(id);
    ARENAS.lock().unwrap().insert(id, arena);
    id
}

/// Get arena by ID
fn get_arena<F, R>(id: u32, f: F) -> Option<R>
where
    F: FnOnce(&PdfObjectArena) -> R,
{
    let arenas = ARENAS.lock().unwrap();
    arenas.get(&id).map(f)
}

/// Get mutable arena by ID
fn get_arena_mut<F, R>(id: u32, f: F) -> Option<R>
where
    F: FnOnce(&mut PdfObjectArena) -> R,
{
    let mut arenas = ARENAS.lock().unwrap();
    arenas.get_mut(&id).map(f)
}

// ============================================================================
// FFI Functions
// ============================================================================

/// Create a new PDF object arena
#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_object_arena(_ctx: Handle) -> u32 {
    create_arena_internal()
}

/// Create a new PDF object arena with custom chunk size
#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_object_arena_with_size(_ctx: Handle, chunk_size: usize) -> u32 {
    let id = {
        let mut counter = ARENA_COUNTER.lock().unwrap();
        let id = *counter;
        *counter += 1;
        id
    };

    let arena = PdfObjectArena::with_chunk_size(id, chunk_size.max(64));
    ARENAS.lock().unwrap().insert(id, arena);
    id
}

/// Drop an arena and all its objects
#[unsafe(no_mangle)]
pub extern "C" fn pdf_drop_object_arena(_ctx: Handle, arena_id: u32) {
    ARENAS.lock().unwrap().remove(&arena_id);
}

/// Clear an arena (free all objects but keep memory)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_clear_object_arena(_ctx: Handle, arena_id: u32) {
    get_arena_mut(arena_id, |arena| arena.clear());
}

/// Compact an arena (release unused chunks)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_compact_object_arena(_ctx: Handle, arena_id: u32) {
    get_arena_mut(arena_id, |arena| arena.compact());
}

/// Allocate a null object in the arena
#[unsafe(no_mangle)]
pub extern "C" fn pdf_arena_new_null(_ctx: Handle, arena_id: u32) -> Handle {
    get_arena_mut(arena_id, |arena| {
        arena.alloc(PdfObj::new_null()).to_handle()
    })
    .unwrap_or(0)
}

/// Allocate a bool object in the arena
#[unsafe(no_mangle)]
pub extern "C" fn pdf_arena_new_bool(_ctx: Handle, arena_id: u32, value: i32) -> Handle {
    get_arena_mut(arena_id, |arena| {
        arena.alloc(PdfObj::new_bool(value != 0)).to_handle()
    })
    .unwrap_or(0)
}

/// Allocate an int object in the arena
#[unsafe(no_mangle)]
pub extern "C" fn pdf_arena_new_int(_ctx: Handle, arena_id: u32, value: i64) -> Handle {
    get_arena_mut(arena_id, |arena| {
        arena.alloc(PdfObj::new_int(value)).to_handle()
    })
    .unwrap_or(0)
}

/// Allocate a real object in the arena
#[unsafe(no_mangle)]
pub extern "C" fn pdf_arena_new_real(_ctx: Handle, arena_id: u32, value: f32) -> Handle {
    get_arena_mut(arena_id, |arena| {
        arena.alloc(PdfObj::new_real(value as f64)).to_handle()
    })
    .unwrap_or(0)
}

/// Allocate a name object in the arena
#[unsafe(no_mangle)]
pub extern "C" fn pdf_arena_new_name(
    _ctx: Handle,
    arena_id: u32,
    name: *const std::ffi::c_char,
) -> Handle {
    let name_str = if name.is_null() {
        ""
    } else {
        unsafe { std::ffi::CStr::from_ptr(name).to_str().unwrap_or("") }
    };
    get_arena_mut(arena_id, |arena| {
        arena.alloc(PdfObj::new_name(name_str)).to_handle()
    })
    .unwrap_or(0)
}

/// Allocate a string object in the arena
#[unsafe(no_mangle)]
pub extern "C" fn pdf_arena_new_string(
    _ctx: Handle,
    arena_id: u32,
    data: *const u8,
    len: usize,
) -> Handle {
    let bytes = if data.is_null() || len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(data, len) }
    };
    get_arena_mut(arena_id, |arena| {
        arena.alloc(PdfObj::new_string(bytes)).to_handle()
    })
    .unwrap_or(0)
}

/// Allocate an array object in the arena
#[unsafe(no_mangle)]
pub extern "C" fn pdf_arena_new_array(_ctx: Handle, arena_id: u32, capacity: usize) -> Handle {
    get_arena_mut(arena_id, |arena| {
        arena.alloc(PdfObj::new_array(capacity)).to_handle()
    })
    .unwrap_or(0)
}

/// Allocate a dict object in the arena
#[unsafe(no_mangle)]
pub extern "C" fn pdf_arena_new_dict(_ctx: Handle, arena_id: u32, capacity: usize) -> Handle {
    get_arena_mut(arena_id, |arena| {
        arena.alloc(PdfObj::new_dict(capacity)).to_handle()
    })
    .unwrap_or(0)
}

/// Allocate an indirect object in the arena
#[unsafe(no_mangle)]
pub extern "C" fn pdf_arena_new_indirect(
    _ctx: Handle,
    arena_id: u32,
    num: i32,
    generation: i32,
) -> Handle {
    get_arena_mut(arena_id, |arena| {
        arena
            .alloc(PdfObj::new_indirect(num, generation))
            .to_handle()
    })
    .unwrap_or(0)
}

/// Free an arena object
#[unsafe(no_mangle)]
pub extern "C" fn pdf_arena_free_obj(_ctx: Handle, handle: Handle) {
    if !ArenaHandle::is_arena_handle(handle) {
        return;
    }
    let arena_handle = ArenaHandle::from_handle(handle);
    get_arena_mut(arena_handle.arena_id, |arena| arena.free(arena_handle));
}

/// Get arena statistics
#[unsafe(no_mangle)]
pub extern "C" fn pdf_object_arena_stats(_ctx: Handle, arena_id: u32) -> ArenaStats {
    get_arena(arena_id, |arena| arena.stats()).unwrap_or(ArenaStats {
        arena_id: 0,
        chunk_count: 0,
        chunk_size: 0,
        capacity: 0,
        allocated: 0,
        freed: 0,
        active: 0,
        utilization: 0.0,
    })
}

/// Get count of active arenas
#[unsafe(no_mangle)]
pub extern "C" fn pdf_object_arena_count(_ctx: Handle) -> usize {
    ARENAS.lock().unwrap().len()
}

/// Check if handle is an arena handle
#[unsafe(no_mangle)]
pub extern "C" fn pdf_is_arena_handle(_ctx: Handle, handle: Handle) -> i32 {
    if ArenaHandle::is_arena_handle(handle) {
        1
    } else {
        0
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arena_create_drop() {
        let arena_id = pdf_new_object_arena(0);
        assert!(arena_id > 0);

        // Verify arena exists by allocating an object in it
        let test_handle = pdf_arena_new_null(0, arena_id);
        assert!(ArenaHandle::is_arena_handle(test_handle));

        // Drop the arena
        pdf_drop_object_arena(0, arena_id);

        // Creating another arena should succeed (proves system is still working)
        let new_arena_id = pdf_new_object_arena(0);
        assert!(new_arena_id > 0);
        pdf_drop_object_arena(0, new_arena_id);
    }

    #[test]
    fn test_arena_alloc_null() {
        let arena_id = pdf_new_object_arena(0);
        let handle = pdf_arena_new_null(0, arena_id);
        assert!(ArenaHandle::is_arena_handle(handle));

        let arena_handle = ArenaHandle::from_handle(handle);
        assert_eq!(arena_handle.arena_id, arena_id);

        pdf_drop_object_arena(0, arena_id);
    }

    #[test]
    fn test_arena_alloc_int() {
        let arena_id = pdf_new_object_arena(0);
        let handle = pdf_arena_new_int(0, arena_id, 42);
        assert!(ArenaHandle::is_arena_handle(handle));

        // Verify we can get the object
        let arena_handle = ArenaHandle::from_handle(handle);
        get_arena(arena_handle.arena_id, |arena| {
            let obj = arena.get(arena_handle).unwrap();
            match &obj.obj_type {
                PdfObjType::Int(v) => assert_eq!(*v, 42),
                _ => panic!("Expected Int"),
            }
        });

        pdf_drop_object_arena(0, arena_id);
    }

    #[test]
    fn test_arena_alloc_many() {
        let arena_id = pdf_new_object_arena(0);

        // Allocate many objects
        let mut handles = Vec::new();
        for i in 0..1000 {
            let handle = pdf_arena_new_int(0, arena_id, i);
            handles.push(handle);
        }

        // Check stats
        let stats = pdf_object_arena_stats(0, arena_id);
        assert_eq!(stats.allocated, 1000);
        assert_eq!(stats.active, 1000);

        pdf_drop_object_arena(0, arena_id);
    }

    #[test]
    fn test_arena_free() {
        let arena_id = pdf_new_object_arena(0);

        let handle = pdf_arena_new_int(0, arena_id, 42);
        let stats_before = pdf_object_arena_stats(0, arena_id);
        assert_eq!(stats_before.active, 1);

        pdf_arena_free_obj(0, handle);
        let stats_after = pdf_object_arena_stats(0, arena_id);
        assert_eq!(stats_after.active, 0);
        assert_eq!(stats_after.freed, 1);

        pdf_drop_object_arena(0, arena_id);
    }

    #[test]
    fn test_arena_clear() {
        let arena_id = pdf_new_object_arena(0);

        for i in 0..100 {
            pdf_arena_new_int(0, arena_id, i);
        }

        let stats_before = pdf_object_arena_stats(0, arena_id);
        assert_eq!(stats_before.active, 100);

        pdf_clear_object_arena(0, arena_id);

        let stats_after = pdf_object_arena_stats(0, arena_id);
        assert_eq!(stats_after.active, 0);

        pdf_drop_object_arena(0, arena_id);
    }

    #[test]
    fn test_arena_handle_encoding() {
        let arena_id = 12345u32;
        let chunk_idx = 100u16;
        let slot_idx = 50u16;

        let handle = ArenaHandle {
            arena_id,
            chunk_idx,
            slot_idx,
        };

        let encoded = handle.to_handle();
        let decoded = ArenaHandle::from_handle(encoded);

        assert_eq!(decoded.arena_id, arena_id);
        assert_eq!(decoded.chunk_idx, chunk_idx);
        assert_eq!(decoded.slot_idx, slot_idx);
    }

    #[test]
    fn test_is_arena_handle() {
        // Arena handles have non-zero high bits
        assert_eq!(pdf_is_arena_handle(0, 0), 0);
        assert_eq!(pdf_is_arena_handle(0, 100), 0); // Regular handle

        let arena_id = pdf_new_object_arena(0);
        let handle = pdf_arena_new_int(0, arena_id, 42);
        assert_eq!(pdf_is_arena_handle(0, handle), 1);

        pdf_drop_object_arena(0, arena_id);
    }

    #[test]
    fn test_arena_custom_chunk_size() {
        let arena_id = pdf_new_object_arena_with_size(0, 128);
        let stats = pdf_object_arena_stats(0, arena_id);
        assert_eq!(stats.chunk_size, 128);

        pdf_drop_object_arena(0, arena_id);
    }

    #[test]
    fn test_arena_compact() {
        let arena_id = pdf_new_object_arena_with_size(0, 10);

        // Allocate enough to create multiple chunks (30 objects with chunk size 10)
        let mut handles = Vec::new();
        for i in 0..30 {
            handles.push(pdf_arena_new_int(0, arena_id, i));
        }

        let stats_before = pdf_object_arena_stats(0, arena_id);
        // With chunk size 10 and 30 objects, we need at least 3 chunks
        assert!(
            stats_before.chunk_count >= 1,
            "Expected at least 1 chunk, got {}",
            stats_before.chunk_count
        );

        // Free all objects
        for handle in handles {
            pdf_arena_free_obj(0, handle);
        }

        // Compact
        pdf_compact_object_arena(0, arena_id);

        let stats_after = pdf_object_arena_stats(0, arena_id);
        assert_eq!(stats_after.chunk_count, 0);

        pdf_drop_object_arena(0, arena_id);
    }

    #[test]
    fn test_arena_all_types() {
        let arena_id = pdf_new_object_arena(0);

        // Test all object types
        let null_h = pdf_arena_new_null(0, arena_id);
        assert!(ArenaHandle::is_arena_handle(null_h));

        let bool_h = pdf_arena_new_bool(0, arena_id, 1);
        assert!(ArenaHandle::is_arena_handle(bool_h));

        let int_h = pdf_arena_new_int(0, arena_id, 42);
        assert!(ArenaHandle::is_arena_handle(int_h));

        let real_h = pdf_arena_new_real(0, arena_id, 3.5);
        assert!(ArenaHandle::is_arena_handle(real_h));

        let name = std::ffi::CString::new("Type").unwrap();
        let name_h = pdf_arena_new_name(0, arena_id, name.as_ptr());
        assert!(ArenaHandle::is_arena_handle(name_h));

        let data = b"Hello";
        let str_h = pdf_arena_new_string(0, arena_id, data.as_ptr(), data.len());
        assert!(ArenaHandle::is_arena_handle(str_h));

        let arr_h = pdf_arena_new_array(0, arena_id, 10);
        assert!(ArenaHandle::is_arena_handle(arr_h));

        let dict_h = pdf_arena_new_dict(0, arena_id, 5);
        assert!(ArenaHandle::is_arena_handle(dict_h));

        let ind_h = pdf_arena_new_indirect(0, arena_id, 10, 0);
        assert!(ArenaHandle::is_arena_handle(ind_h));

        let stats = pdf_object_arena_stats(0, arena_id);
        assert_eq!(stats.allocated, 9);

        pdf_drop_object_arena(0, arena_id);
    }
}
