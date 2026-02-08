//! C FFI for memory pool - MuPDF compatible
//! Safe Rust implementation of fz_pool

use super::{Handle, HandleStore};
use std::sync::LazyLock;

/// Default block size for pool allocations
const DEFAULT_BLOCK_SIZE: usize = 4096;

/// Memory block in a pool
#[derive(Debug)]
pub struct PoolBlock {
    /// Block data
    data: Vec<u8>,
    /// Current position in block
    pos: usize,
}

impl PoolBlock {
    fn new(size: usize) -> Self {
        Self {
            data: vec![0u8; size],
            pos: 0,
        }
    }

    fn available(&self) -> usize {
        self.data.len() - self.pos
    }

    fn allocate(&mut self, size: usize, align: usize) -> Option<usize> {
        // Calculate the base address of the data buffer
        let base_addr = self.data.as_ptr() as usize;

        // Calculate the address at current position
        let current_addr = base_addr + self.pos;

        // Align the address (not just the offset)
        let aligned_addr = (current_addr + align - 1) & !(align - 1);

        // Calculate the new offset
        let aligned_pos = aligned_addr - base_addr;

        if aligned_pos + size <= self.data.len() {
            let offset = aligned_pos;
            self.pos = aligned_pos + size;
            Some(offset)
        } else {
            None
        }
    }
}

/// Memory pool structure
#[derive(Debug)]
pub struct Pool {
    /// Block size for new allocations
    pub block_size: usize,
    /// All allocated blocks
    pub blocks: Vec<PoolBlock>,
    /// Total bytes allocated
    pub total_allocated: usize,
    /// Total bytes used
    pub total_used: usize,
    /// High water mark (max used)
    pub high_water: usize,
    /// Number of allocations
    pub alloc_count: usize,
    /// Pool name (for debugging)
    pub name: String,
}

impl Default for Pool {
    fn default() -> Self {
        Self {
            block_size: DEFAULT_BLOCK_SIZE,
            blocks: Vec::new(),
            total_allocated: 0,
            total_used: 0,
            high_water: 0,
            alloc_count: 0,
            name: String::new(),
        }
    }
}

impl Pool {
    /// Allocate memory from pool
    fn alloc(&mut self, size: usize, align: usize) -> Option<*mut u8> {
        if size == 0 {
            return Some(std::ptr::null_mut());
        }

        let align = align.max(1);

        // Try to allocate from existing blocks
        for block in &mut self.blocks {
            if let Some(offset) = block.allocate(size, align) {
                self.total_used += size;
                self.alloc_count += 1;
                if self.total_used > self.high_water {
                    self.high_water = self.total_used;
                }
                return Some(block.data.as_mut_ptr().wrapping_add(offset));
            }
        }

        // Need a new block
        let new_block_size = self.block_size.max(size + align);
        let mut new_block = PoolBlock::new(new_block_size);
        self.total_allocated += new_block_size;

        if let Some(offset) = new_block.allocate(size, align) {
            let ptr = new_block.data.as_mut_ptr().wrapping_add(offset);
            self.blocks.push(new_block);
            self.total_used += size;
            self.alloc_count += 1;
            if self.total_used > self.high_water {
                self.high_water = self.total_used;
            }
            return Some(ptr);
        }

        None
    }

    /// Reset pool (free all memory for reuse)
    fn reset(&mut self) {
        for block in &mut self.blocks {
            block.pos = 0;
        }
        self.total_used = 0;
        self.alloc_count = 0;
    }

    /// Shrink pool (release unused blocks)
    fn shrink(&mut self) {
        // Keep only blocks that have been used
        self.blocks.retain(|b| b.pos > 0);

        // Recalculate total allocated
        self.total_allocated = self.blocks.iter().map(|b| b.data.len()).sum();
    }
}

/// Global pool storage
pub static POOLS: LazyLock<HandleStore<Pool>> = LazyLock::new(HandleStore::new);

// ============================================================================
// Pool Creation
// ============================================================================

/// Create a new memory pool
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_pool(_ctx: Handle) -> Handle {
    POOLS.insert(Pool::default())
}

/// Create a new memory pool with specified block size
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_pool_with_size(_ctx: Handle, block_size: usize) -> Handle {
    let pool = Pool {
        block_size: block_size.max(64),
        ..Default::default()
    };
    POOLS.insert(pool)
}

/// Create a named pool (for debugging)
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_pool_named(_ctx: Handle, name: *const std::ffi::c_char) -> Handle {
    let pool_name = if name.is_null() {
        String::new()
    } else {
        let c_str = unsafe { std::ffi::CStr::from_ptr(name) };
        c_str.to_str().unwrap_or("").to_string()
    };

    let pool = Pool {
        name: pool_name,
        ..Default::default()
    };
    POOLS.insert(pool)
}

// ============================================================================
// Pool Allocation
// ============================================================================

/// Allocate memory from pool
#[unsafe(no_mangle)]
pub extern "C" fn fz_pool_alloc(_ctx: Handle, pool: Handle, size: usize) -> *mut u8 {
    if let Some(p) = POOLS.get(pool) {
        if let Ok(mut guard) = p.lock() {
            return guard.alloc(size, 8).unwrap_or(std::ptr::null_mut());
        }
    }
    std::ptr::null_mut()
}

/// Allocate aligned memory from pool
#[unsafe(no_mangle)]
pub extern "C" fn fz_pool_alloc_aligned(
    _ctx: Handle,
    pool: Handle,
    size: usize,
    align: usize,
) -> *mut u8 {
    if let Some(p) = POOLS.get(pool) {
        if let Ok(mut guard) = p.lock() {
            return guard.alloc(size, align).unwrap_or(std::ptr::null_mut());
        }
    }
    std::ptr::null_mut()
}

/// Allocate zeroed memory from pool
#[unsafe(no_mangle)]
pub extern "C" fn fz_pool_calloc(_ctx: Handle, pool: Handle, count: usize, size: usize) -> *mut u8 {
    let total_size = count.saturating_mul(size);
    if let Some(p) = POOLS.get(pool) {
        if let Ok(mut guard) = p.lock() {
            if let Some(ptr) = guard.alloc(total_size, 8) {
                // Memory is already zeroed in our implementation
                return ptr;
            }
        }
    }
    std::ptr::null_mut()
}

/// Duplicate string into pool
#[unsafe(no_mangle)]
pub extern "C" fn fz_pool_strdup(
    _ctx: Handle,
    pool: Handle,
    s: *const std::ffi::c_char,
) -> *mut std::ffi::c_char {
    if s.is_null() {
        return std::ptr::null_mut();
    }

    // Calculate string length manually (like strlen)
    let mut len = 0usize;
    unsafe {
        while *s.add(len) != 0 {
            len += 1;
        }
    }
    let size = len + 1;

    if let Some(p) = POOLS.get(pool) {
        if let Ok(mut guard) = p.lock() {
            if let Some(ptr) = guard.alloc(size, 1) {
                unsafe {
                    std::ptr::copy_nonoverlapping(s as *const u8, ptr, size);
                }
                return ptr as *mut std::ffi::c_char;
            }
        }
    }
    std::ptr::null_mut()
}

// ============================================================================
// Pool Management
// ============================================================================

/// Reset pool (free all allocations for reuse)
#[unsafe(no_mangle)]
pub extern "C" fn fz_pool_reset(_ctx: Handle, pool: Handle) {
    if let Some(p) = POOLS.get(pool) {
        if let Ok(mut guard) = p.lock() {
            guard.reset();
        }
    }
}

/// Shrink pool (release unused blocks)
#[unsafe(no_mangle)]
pub extern "C" fn fz_pool_shrink(_ctx: Handle, pool: Handle) {
    if let Some(p) = POOLS.get(pool) {
        if let Ok(mut guard) = p.lock() {
            guard.shrink();
        }
    }
}

/// Set block size for future allocations
#[unsafe(no_mangle)]
pub extern "C" fn fz_pool_set_block_size(_ctx: Handle, pool: Handle, size: usize) {
    if let Some(p) = POOLS.get(pool) {
        if let Ok(mut guard) = p.lock() {
            guard.block_size = size.max(64);
        }
    }
}

// ============================================================================
// Pool Statistics
// ============================================================================

/// Get total allocated bytes
#[unsafe(no_mangle)]
pub extern "C" fn fz_pool_allocated(_ctx: Handle, pool: Handle) -> usize {
    if let Some(p) = POOLS.get(pool) {
        if let Ok(guard) = p.lock() {
            return guard.total_allocated;
        }
    }
    0
}

/// Get total used bytes
#[unsafe(no_mangle)]
pub extern "C" fn fz_pool_used(_ctx: Handle, pool: Handle) -> usize {
    if let Some(p) = POOLS.get(pool) {
        if let Ok(guard) = p.lock() {
            return guard.total_used;
        }
    }
    0
}

/// Get high water mark
#[unsafe(no_mangle)]
pub extern "C" fn fz_pool_high_water(_ctx: Handle, pool: Handle) -> usize {
    if let Some(p) = POOLS.get(pool) {
        if let Ok(guard) = p.lock() {
            return guard.high_water;
        }
    }
    0
}

/// Get allocation count
#[unsafe(no_mangle)]
pub extern "C" fn fz_pool_alloc_count(_ctx: Handle, pool: Handle) -> usize {
    if let Some(p) = POOLS.get(pool) {
        if let Ok(guard) = p.lock() {
            return guard.alloc_count;
        }
    }
    0
}

/// Get number of blocks
#[unsafe(no_mangle)]
pub extern "C" fn fz_pool_block_count(_ctx: Handle, pool: Handle) -> usize {
    if let Some(p) = POOLS.get(pool) {
        if let Ok(guard) = p.lock() {
            return guard.blocks.len();
        }
    }
    0
}

/// Get available space in current blocks
#[unsafe(no_mangle)]
pub extern "C" fn fz_pool_available(_ctx: Handle, pool: Handle) -> usize {
    if let Some(p) = POOLS.get(pool) {
        if let Ok(guard) = p.lock() {
            return guard.blocks.iter().map(|b| b.available()).sum();
        }
    }
    0
}

/// Get fragmentation ratio (0.0 to 1.0)
#[unsafe(no_mangle)]
pub extern "C" fn fz_pool_fragmentation(_ctx: Handle, pool: Handle) -> f32 {
    if let Some(p) = POOLS.get(pool) {
        if let Ok(guard) = p.lock() {
            if guard.total_allocated == 0 {
                return 0.0;
            }
            let wasted = guard.total_allocated - guard.total_used;
            return wasted as f32 / guard.total_allocated as f32;
        }
    }
    0.0
}

// ============================================================================
// Reference Counting
// ============================================================================

/// Keep pool reference
#[unsafe(no_mangle)]
pub extern "C" fn fz_keep_pool(_ctx: Handle, pool: Handle) -> Handle {
    POOLS.keep(pool)
}

/// Drop pool reference
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_pool(_ctx: Handle, pool: Handle) {
    POOLS.remove(pool);
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_pool() {
        let pool = fz_new_pool(0);
        assert!(pool > 0);

        assert_eq!(fz_pool_allocated(0, pool), 0);
        assert_eq!(fz_pool_used(0, pool), 0);

        fz_drop_pool(0, pool);
    }

    #[test]
    fn test_pool_alloc() {
        let pool = fz_new_pool_with_size(0, 1024);

        let ptr1 = fz_pool_alloc(0, pool, 100);
        assert!(!ptr1.is_null());

        let ptr2 = fz_pool_alloc(0, pool, 200);
        assert!(!ptr2.is_null());

        assert_eq!(fz_pool_alloc_count(0, pool), 2);
        assert!(fz_pool_used(0, pool) >= 300);

        fz_drop_pool(0, pool);
    }

    #[test]
    fn test_pool_aligned_alloc() {
        let pool = fz_new_pool(0);

        let ptr = fz_pool_alloc_aligned(0, pool, 64, 64);
        assert!(!ptr.is_null());
        assert_eq!(ptr as usize % 64, 0);

        fz_drop_pool(0, pool);
    }

    #[test]
    fn test_pool_reset() {
        let pool = fz_new_pool(0);

        fz_pool_alloc(0, pool, 100);
        fz_pool_alloc(0, pool, 200);

        assert!(fz_pool_used(0, pool) >= 300);

        fz_pool_reset(0, pool);

        assert_eq!(fz_pool_used(0, pool), 0);
        assert!(fz_pool_allocated(0, pool) > 0); // Blocks retained

        fz_drop_pool(0, pool);
    }

    #[test]
    fn test_pool_shrink() {
        let pool = fz_new_pool_with_size(0, 256);

        // Allocate multiple blocks
        for _ in 0..5 {
            fz_pool_alloc(0, pool, 256);
        }

        let blocks_before = fz_pool_block_count(0, pool);
        assert!(blocks_before >= 5);

        fz_pool_reset(0, pool);
        fz_pool_shrink(0, pool);

        // All blocks should be released since none are in use
        assert_eq!(fz_pool_block_count(0, pool), 0);

        fz_drop_pool(0, pool);
    }

    #[test]
    fn test_pool_calloc() {
        let pool = fz_new_pool(0);

        let ptr = fz_pool_calloc(0, pool, 10, 8);
        assert!(!ptr.is_null());

        // Check memory is zeroed
        let slice = unsafe { std::slice::from_raw_parts(ptr, 80) };
        assert!(slice.iter().all(|&b| b == 0));

        fz_drop_pool(0, pool);
    }

    #[test]
    fn test_pool_strdup() {
        let pool = fz_new_pool(0);

        let s = c"Hello, World!";
        let dup = fz_pool_strdup(0, pool, s.as_ptr());
        assert!(!dup.is_null());

        let dup_str = unsafe { std::ffi::CStr::from_ptr(dup) };
        assert_eq!(dup_str.to_str().unwrap(), "Hello, World!");

        fz_drop_pool(0, pool);
    }

    #[test]
    fn test_pool_fragmentation() {
        let pool = fz_new_pool_with_size(0, 1024);

        // Small allocations in large blocks = high fragmentation
        fz_pool_alloc(0, pool, 10);

        let frag = fz_pool_fragmentation(0, pool);
        assert!(frag > 0.9); // >90% wasted

        fz_drop_pool(0, pool);
    }

    #[test]
    fn test_named_pool() {
        let name = c"test_pool";
        let pool = fz_new_pool_named(0, name.as_ptr());
        assert!(pool > 0);

        fz_drop_pool(0, pool);
    }
}
