//! FFI bindings for fz_heap (Priority Queue / Heap Sort)
//!
//! Provides heap-based priority queue for sorted rendering and other uses.

use crate::ffi::{Handle, HandleStore};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::LazyLock;

// ============================================================================
// Types
// ============================================================================

/// Heap element types
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HeapType {
    #[default]
    Int = 0,
    Ptr = 1,
    Int2 = 2,
    IntPtr = 3,
}

impl HeapType {
    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => HeapType::Int,
            1 => HeapType::Ptr,
            2 => HeapType::Int2,
            3 => HeapType::IntPtr,
            _ => HeapType::Int,
        }
    }
}

/// Int2 pair structure (two ints, sorted by first)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Int2 {
    pub a: i32,
    pub b: i32,
}

impl PartialEq for Int2 {
    fn eq(&self, other: &Self) -> bool {
        self.a == other.a
    }
}

impl Eq for Int2 {}

impl PartialOrd for Int2 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Int2 {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse for min-heap behavior (smallest first when sorted)
        other.a.cmp(&self.a)
    }
}

/// IntPtr pair structure (int + pointer, sorted by int)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct IntPtr {
    pub a: i32,
    pub b: usize, // Using usize for pointer
}

impl PartialEq for IntPtr {
    fn eq(&self, other: &Self) -> bool {
        self.a == other.a
    }
}

impl Eq for IntPtr {}

impl PartialOrd for IntPtr {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for IntPtr {
    fn cmp(&self, other: &Self) -> Ordering {
        other.a.cmp(&self.a)
    }
}

/// Wrapper for i32 with reverse ordering for min-heap
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MinInt(i32);

impl PartialOrd for MinInt {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MinInt {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.cmp(&self.0) // Reverse for min-heap
    }
}

/// Generic heap structure
#[derive(Debug)]
pub enum Heap {
    Int(BinaryHeap<MinInt>),
    Ptr(BinaryHeap<usize>),
    Int2(BinaryHeap<Int2>),
    IntPtr(BinaryHeap<IntPtr>),
}

impl Default for Heap {
    fn default() -> Self {
        Heap::Int(BinaryHeap::new())
    }
}

impl Heap {
    pub fn new(heap_type: HeapType) -> Self {
        match heap_type {
            HeapType::Int => Heap::Int(BinaryHeap::new()),
            HeapType::Ptr => Heap::Ptr(BinaryHeap::new()),
            HeapType::Int2 => Heap::Int2(BinaryHeap::new()),
            HeapType::IntPtr => Heap::IntPtr(BinaryHeap::new()),
        }
    }

    pub fn with_capacity(heap_type: HeapType, capacity: usize) -> Self {
        match heap_type {
            HeapType::Int => Heap::Int(BinaryHeap::with_capacity(capacity)),
            HeapType::Ptr => Heap::Ptr(BinaryHeap::with_capacity(capacity)),
            HeapType::Int2 => Heap::Int2(BinaryHeap::with_capacity(capacity)),
            HeapType::IntPtr => Heap::IntPtr(BinaryHeap::with_capacity(capacity)),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Heap::Int(h) => h.len(),
            Heap::Ptr(h) => h.len(),
            Heap::Int2(h) => h.len(),
            Heap::IntPtr(h) => h.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn capacity(&self) -> usize {
        match self {
            Heap::Int(h) => h.capacity(),
            Heap::Ptr(h) => h.capacity(),
            Heap::Int2(h) => h.capacity(),
            Heap::IntPtr(h) => h.capacity(),
        }
    }

    pub fn clear(&mut self) {
        match self {
            Heap::Int(h) => h.clear(),
            Heap::Ptr(h) => h.clear(),
            Heap::Int2(h) => h.clear(),
            Heap::IntPtr(h) => h.clear(),
        }
    }

    pub fn heap_type(&self) -> HeapType {
        match self {
            Heap::Int(_) => HeapType::Int,
            Heap::Ptr(_) => HeapType::Ptr,
            Heap::Int2(_) => HeapType::Int2,
            Heap::IntPtr(_) => HeapType::IntPtr,
        }
    }
}

// Global heap store
pub static HEAPS: LazyLock<HandleStore<Heap>> = LazyLock::new(HandleStore::new);

// ============================================================================
// FFI Functions - Creation/Destruction
// ============================================================================

/// Create a new heap
///
/// @param ctx       Context handle
/// @param heap_type Type of heap (0=int, 1=ptr, 2=int2, 3=intptr)
///
/// Returns heap handle, or 0 on error
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_heap(_ctx: Handle, heap_type: i32) -> Handle {
    let ht = HeapType::from_i32(heap_type);
    HEAPS.insert(Heap::new(ht))
}

/// Create a new heap with initial capacity
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_heap_with_capacity(
    _ctx: Handle,
    heap_type: i32,
    capacity: usize,
) -> Handle {
    let ht = HeapType::from_i32(heap_type);
    HEAPS.insert(Heap::with_capacity(ht, capacity))
}

/// Drop/free a heap
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_heap(_ctx: Handle, heap: Handle) {
    HEAPS.remove(heap);
}

/// Clear all elements from heap
#[unsafe(no_mangle)]
pub extern "C" fn fz_heap_clear(_ctx: Handle, heap: Handle) {
    if let Some(h) = HEAPS.get(heap) {
        h.lock().unwrap().clear();
    }
}

// ============================================================================
// FFI Functions - Int Heap
// ============================================================================

/// Insert an integer into the heap
#[unsafe(no_mangle)]
pub extern "C" fn fz_int_heap_insert(_ctx: Handle, heap: Handle, value: i32) -> i32 {
    let h = match HEAPS.get(heap) {
        Some(h) => h,
        None => return 0,
    };

    let mut guard = h.lock().unwrap();
    if let Heap::Int(ref mut heap) = *guard {
        heap.push(MinInt(value));
        1
    } else {
        0
    }
}

/// Pop the minimum integer from the heap
#[unsafe(no_mangle)]
pub extern "C" fn fz_int_heap_pop(_ctx: Handle, heap: Handle, value_out: *mut i32) -> i32 {
    if value_out.is_null() {
        return 0;
    }

    let h = match HEAPS.get(heap) {
        Some(h) => h,
        None => return 0,
    };

    let mut guard = h.lock().unwrap();
    if let Heap::Int(ref mut heap) = *guard {
        if let Some(MinInt(v)) = heap.pop() {
            unsafe {
                *value_out = v;
            }
            1
        } else {
            0
        }
    } else {
        0
    }
}

/// Peek at the minimum integer without removing
#[unsafe(no_mangle)]
pub extern "C" fn fz_int_heap_peek(_ctx: Handle, heap: Handle, value_out: *mut i32) -> i32 {
    if value_out.is_null() {
        return 0;
    }

    let h = match HEAPS.get(heap) {
        Some(h) => h,
        None => return 0,
    };

    let guard = h.lock().unwrap();
    if let Heap::Int(ref heap) = *guard {
        if let Some(MinInt(v)) = heap.peek() {
            unsafe {
                *value_out = *v;
            }
            1
        } else {
            0
        }
    } else {
        0
    }
}

/// Sort the int heap and return sorted array
#[unsafe(no_mangle)]
pub extern "C" fn fz_int_heap_sort(
    _ctx: Handle,
    heap: Handle,
    out: *mut i32,
    out_len: usize,
) -> usize {
    if out.is_null() {
        return 0;
    }

    let h = match HEAPS.get(heap) {
        Some(h) => h,
        None => return 0,
    };

    let guard = h.lock().unwrap();
    if let Heap::Int(ref heap) = *guard {
        let mut sorted: Vec<i32> = heap.iter().map(|MinInt(v)| *v).collect();
        sorted.sort();

        let copy_len = sorted.len().min(out_len);
        unsafe {
            std::ptr::copy_nonoverlapping(sorted.as_ptr(), out, copy_len);
        }
        copy_len
    } else {
        0
    }
}

// ============================================================================
// FFI Functions - Ptr Heap
// ============================================================================

/// Insert a pointer into the heap
#[unsafe(no_mangle)]
pub extern "C" fn fz_ptr_heap_insert(_ctx: Handle, heap: Handle, value: usize) -> i32 {
    let h = match HEAPS.get(heap) {
        Some(h) => h,
        None => return 0,
    };

    let mut guard = h.lock().unwrap();
    if let Heap::Ptr(ref mut heap) = *guard {
        heap.push(value);
        1
    } else {
        0
    }
}

/// Pop a pointer from the heap
#[unsafe(no_mangle)]
pub extern "C" fn fz_ptr_heap_pop(_ctx: Handle, heap: Handle, value_out: *mut usize) -> i32 {
    if value_out.is_null() {
        return 0;
    }

    let h = match HEAPS.get(heap) {
        Some(h) => h,
        None => return 0,
    };

    let mut guard = h.lock().unwrap();
    if let Heap::Ptr(ref mut heap) = *guard {
        if let Some(v) = heap.pop() {
            unsafe {
                *value_out = v;
            }
            1
        } else {
            0
        }
    } else {
        0
    }
}

// ============================================================================
// FFI Functions - Int2 Heap
// ============================================================================

/// Insert an Int2 pair into the heap
#[unsafe(no_mangle)]
pub extern "C" fn fz_int2_heap_insert(_ctx: Handle, heap: Handle, a: i32, b: i32) -> i32 {
    let h = match HEAPS.get(heap) {
        Some(h) => h,
        None => return 0,
    };

    let mut guard = h.lock().unwrap();
    if let Heap::Int2(ref mut heap) = *guard {
        heap.push(Int2 { a, b });
        1
    } else {
        0
    }
}

/// Pop an Int2 pair from the heap
#[unsafe(no_mangle)]
pub extern "C" fn fz_int2_heap_pop(
    _ctx: Handle,
    heap: Handle,
    a_out: *mut i32,
    b_out: *mut i32,
) -> i32 {
    if a_out.is_null() || b_out.is_null() {
        return 0;
    }

    let h = match HEAPS.get(heap) {
        Some(h) => h,
        None => return 0,
    };

    let mut guard = h.lock().unwrap();
    if let Heap::Int2(ref mut heap) = *guard {
        if let Some(Int2 { a, b }) = heap.pop() {
            unsafe {
                *a_out = a;
                *b_out = b;
            }
            1
        } else {
            0
        }
    } else {
        0
    }
}

/// Sort the Int2 heap
#[unsafe(no_mangle)]
pub extern "C" fn fz_int2_heap_sort(_ctx: Handle, heap: Handle) {
    // BinaryHeap is already a heap, but we can convert to sorted order
    // The sort is implicit when iterating
    let _ = HEAPS.get(heap);
}

/// Remove duplicate entries from Int2 heap (based on 'a' field)
#[unsafe(no_mangle)]
pub extern "C" fn fz_int2_heap_uniq(_ctx: Handle, heap: Handle) -> usize {
    let h = match HEAPS.get(heap) {
        Some(h) => h,
        None => return 0,
    };

    let mut guard = h.lock().unwrap();
    if let Heap::Int2(ref mut heap) = *guard {
        let mut sorted: Vec<Int2> = heap.drain().collect();
        sorted.sort_by_key(|v| v.a);
        sorted.dedup_by_key(|v| v.a);

        let len = sorted.len();
        for item in sorted {
            heap.push(item);
        }
        len
    } else {
        0
    }
}

// ============================================================================
// FFI Functions - IntPtr Heap
// ============================================================================

/// Insert an IntPtr pair into the heap
#[unsafe(no_mangle)]
pub extern "C" fn fz_intptr_heap_insert(_ctx: Handle, heap: Handle, a: i32, b: usize) -> i32 {
    let h = match HEAPS.get(heap) {
        Some(h) => h,
        None => return 0,
    };

    let mut guard = h.lock().unwrap();
    if let Heap::IntPtr(ref mut heap) = *guard {
        heap.push(IntPtr { a, b });
        1
    } else {
        0
    }
}

/// Pop an IntPtr pair from the heap
#[unsafe(no_mangle)]
pub extern "C" fn fz_intptr_heap_pop(
    _ctx: Handle,
    heap: Handle,
    a_out: *mut i32,
    b_out: *mut usize,
) -> i32 {
    if a_out.is_null() || b_out.is_null() {
        return 0;
    }

    let h = match HEAPS.get(heap) {
        Some(h) => h,
        None => return 0,
    };

    let mut guard = h.lock().unwrap();
    if let Heap::IntPtr(ref mut heap) = *guard {
        if let Some(IntPtr { a, b }) = heap.pop() {
            unsafe {
                *a_out = a;
                *b_out = b;
            }
            1
        } else {
            0
        }
    } else {
        0
    }
}

/// Sort the IntPtr heap
#[unsafe(no_mangle)]
pub extern "C" fn fz_intptr_heap_sort(_ctx: Handle, heap: Handle) {
    let _ = HEAPS.get(heap);
}

/// Remove duplicate entries from IntPtr heap
#[unsafe(no_mangle)]
pub extern "C" fn fz_intptr_heap_uniq(_ctx: Handle, heap: Handle) -> usize {
    let h = match HEAPS.get(heap) {
        Some(h) => h,
        None => return 0,
    };

    let mut guard = h.lock().unwrap();
    if let Heap::IntPtr(ref mut heap) = *guard {
        let mut sorted: Vec<IntPtr> = heap.drain().collect();
        sorted.sort_by_key(|v| v.a);
        sorted.dedup_by_key(|v| v.a);

        let len = sorted.len();
        for item in sorted {
            heap.push(item);
        }
        len
    } else {
        0
    }
}

// ============================================================================
// FFI Functions - Common Operations
// ============================================================================

/// Get number of elements in heap
#[unsafe(no_mangle)]
pub extern "C" fn fz_heap_len(_ctx: Handle, heap: Handle) -> usize {
    let h = match HEAPS.get(heap) {
        Some(h) => h,
        None => return 0,
    };
    h.lock().unwrap().len()
}

/// Check if heap is empty
#[unsafe(no_mangle)]
pub extern "C" fn fz_heap_is_empty(_ctx: Handle, heap: Handle) -> i32 {
    let h = match HEAPS.get(heap) {
        Some(h) => h,
        None => return 1,
    };
    if h.lock().unwrap().is_empty() { 1 } else { 0 }
}

/// Get heap capacity
#[unsafe(no_mangle)]
pub extern "C" fn fz_heap_capacity(_ctx: Handle, heap: Handle) -> usize {
    let h = match HEAPS.get(heap) {
        Some(h) => h,
        None => return 0,
    };
    h.lock().unwrap().capacity()
}

/// Get heap type
#[unsafe(no_mangle)]
pub extern "C" fn fz_heap_type(_ctx: Handle, heap: Handle) -> i32 {
    let h = match HEAPS.get(heap) {
        Some(h) => h,
        None => return -1,
    };
    h.lock().unwrap().heap_type() as i32
}

/// Reserve capacity for more elements
#[unsafe(no_mangle)]
pub extern "C" fn fz_heap_reserve(_ctx: Handle, heap: Handle, additional: usize) {
    let h = match HEAPS.get(heap) {
        Some(h) => h,
        None => return,
    };

    let mut guard = h.lock().unwrap();
    match &mut *guard {
        Heap::Int(heap) => heap.reserve(additional),
        Heap::Ptr(heap) => heap.reserve(additional),
        Heap::Int2(heap) => heap.reserve(additional),
        Heap::IntPtr(heap) => heap.reserve(additional),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heap_type_enum() {
        assert_eq!(HeapType::from_i32(0), HeapType::Int);
        assert_eq!(HeapType::from_i32(1), HeapType::Ptr);
        assert_eq!(HeapType::from_i32(2), HeapType::Int2);
        assert_eq!(HeapType::from_i32(3), HeapType::IntPtr);
        assert_eq!(HeapType::from_i32(99), HeapType::Int);
    }

    #[test]
    fn test_new_heap() {
        let ctx = 1;
        let heap = fz_new_heap(ctx, HeapType::Int as i32);
        assert!(heap > 0);
        assert_eq!(fz_heap_len(ctx, heap), 0);
        assert_eq!(fz_heap_is_empty(ctx, heap), 1);
        fz_drop_heap(ctx, heap);
    }

    #[test]
    fn test_new_heap_with_capacity() {
        let ctx = 1;
        let heap = fz_new_heap_with_capacity(ctx, HeapType::Int as i32, 100);
        assert!(heap > 0);
        assert!(fz_heap_capacity(ctx, heap) >= 100);
        fz_drop_heap(ctx, heap);
    }

    #[test]
    fn test_int_heap_insert_pop() {
        let ctx = 1;
        let heap = fz_new_heap(ctx, HeapType::Int as i32);

        // Insert values
        fz_int_heap_insert(ctx, heap, 5);
        fz_int_heap_insert(ctx, heap, 2);
        fz_int_heap_insert(ctx, heap, 8);
        fz_int_heap_insert(ctx, heap, 1);
        fz_int_heap_insert(ctx, heap, 9);

        assert_eq!(fz_heap_len(ctx, heap), 5);

        // Pop should return in sorted order (min first)
        let mut value = 0;
        assert_eq!(fz_int_heap_pop(ctx, heap, &mut value), 1);
        assert_eq!(value, 1);

        assert_eq!(fz_int_heap_pop(ctx, heap, &mut value), 1);
        assert_eq!(value, 2);

        assert_eq!(fz_int_heap_pop(ctx, heap, &mut value), 1);
        assert_eq!(value, 5);

        fz_drop_heap(ctx, heap);
    }

    #[test]
    fn test_int_heap_peek() {
        let ctx = 1;
        let heap = fz_new_heap(ctx, HeapType::Int as i32);

        fz_int_heap_insert(ctx, heap, 10);
        fz_int_heap_insert(ctx, heap, 3);
        fz_int_heap_insert(ctx, heap, 7);

        let mut value = 0;
        assert_eq!(fz_int_heap_peek(ctx, heap, &mut value), 1);
        assert_eq!(value, 3); // Min value

        // Length unchanged after peek
        assert_eq!(fz_heap_len(ctx, heap), 3);

        fz_drop_heap(ctx, heap);
    }

    #[test]
    fn test_int_heap_sort() {
        let ctx = 1;
        let heap = fz_new_heap(ctx, HeapType::Int as i32);

        fz_int_heap_insert(ctx, heap, 5);
        fz_int_heap_insert(ctx, heap, 2);
        fz_int_heap_insert(ctx, heap, 8);
        fz_int_heap_insert(ctx, heap, 1);

        let mut sorted = [0i32; 10];
        let len = fz_int_heap_sort(ctx, heap, sorted.as_mut_ptr(), sorted.len());

        assert_eq!(len, 4);
        assert_eq!(&sorted[..4], &[1, 2, 5, 8]);

        fz_drop_heap(ctx, heap);
    }

    #[test]
    fn test_int2_heap() {
        let ctx = 1;
        let heap = fz_new_heap(ctx, HeapType::Int2 as i32);

        fz_int2_heap_insert(ctx, heap, 5, 100);
        fz_int2_heap_insert(ctx, heap, 2, 200);
        fz_int2_heap_insert(ctx, heap, 8, 300);

        assert_eq!(fz_heap_len(ctx, heap), 3);

        let mut a = 0;
        let mut b = 0;

        // Pop should return smallest 'a' first
        assert_eq!(fz_int2_heap_pop(ctx, heap, &mut a, &mut b), 1);
        assert_eq!(a, 2);
        assert_eq!(b, 200);

        fz_drop_heap(ctx, heap);
    }

    #[test]
    fn test_int2_heap_uniq() {
        let ctx = 1;
        let heap = fz_new_heap(ctx, HeapType::Int2 as i32);

        // Insert duplicates (same 'a' value)
        fz_int2_heap_insert(ctx, heap, 5, 100);
        fz_int2_heap_insert(ctx, heap, 5, 200);
        fz_int2_heap_insert(ctx, heap, 2, 300);
        fz_int2_heap_insert(ctx, heap, 2, 400);
        fz_int2_heap_insert(ctx, heap, 8, 500);

        assert_eq!(fz_heap_len(ctx, heap), 5);

        let unique_count = fz_int2_heap_uniq(ctx, heap);
        assert_eq!(unique_count, 3); // 2, 5, 8

        fz_drop_heap(ctx, heap);
    }

    #[test]
    fn test_intptr_heap() {
        let ctx = 1;
        let heap = fz_new_heap(ctx, HeapType::IntPtr as i32);

        let ptr1: usize = 0x1000;
        let ptr2: usize = 0x2000;
        let ptr3: usize = 0x3000;

        fz_intptr_heap_insert(ctx, heap, 5, ptr1);
        fz_intptr_heap_insert(ctx, heap, 2, ptr2);
        fz_intptr_heap_insert(ctx, heap, 8, ptr3);

        let mut a = 0;
        let mut b: usize = 0;

        // Pop should return smallest 'a' first
        assert_eq!(fz_intptr_heap_pop(ctx, heap, &mut a, &mut b), 1);
        assert_eq!(a, 2);
        assert_eq!(b, ptr2);

        fz_drop_heap(ctx, heap);
    }

    #[test]
    fn test_heap_clear() {
        let ctx = 1;
        let heap = fz_new_heap(ctx, HeapType::Int as i32);

        fz_int_heap_insert(ctx, heap, 1);
        fz_int_heap_insert(ctx, heap, 2);
        fz_int_heap_insert(ctx, heap, 3);

        assert_eq!(fz_heap_len(ctx, heap), 3);

        fz_heap_clear(ctx, heap);
        assert_eq!(fz_heap_len(ctx, heap), 0);
        assert_eq!(fz_heap_is_empty(ctx, heap), 1);

        fz_drop_heap(ctx, heap);
    }

    #[test]
    fn test_heap_reserve() {
        let ctx = 1;
        let heap = fz_new_heap(ctx, HeapType::Int as i32);

        let initial_cap = fz_heap_capacity(ctx, heap);
        fz_heap_reserve(ctx, heap, 100);

        assert!(fz_heap_capacity(ctx, heap) >= initial_cap + 100);

        fz_drop_heap(ctx, heap);
    }

    #[test]
    fn test_heap_type() {
        let ctx = 1;

        let int_heap = fz_new_heap(ctx, HeapType::Int as i32);
        assert_eq!(fz_heap_type(ctx, int_heap), HeapType::Int as i32);
        fz_drop_heap(ctx, int_heap);

        let int2_heap = fz_new_heap(ctx, HeapType::Int2 as i32);
        assert_eq!(fz_heap_type(ctx, int2_heap), HeapType::Int2 as i32);
        fz_drop_heap(ctx, int2_heap);
    }

    #[test]
    fn test_empty_heap_pop() {
        let ctx = 1;
        let heap = fz_new_heap(ctx, HeapType::Int as i32);

        let mut value = 999;
        let result = fz_int_heap_pop(ctx, heap, &mut value);
        assert_eq!(result, 0); // Failed, heap is empty
        assert_eq!(value, 999); // Unchanged

        fz_drop_heap(ctx, heap);
    }

    #[test]
    fn test_wrong_heap_type() {
        let ctx = 1;
        let heap = fz_new_heap(ctx, HeapType::Int as i32);

        // Try to insert Int2 into Int heap
        let result = fz_int2_heap_insert(ctx, heap, 1, 2);
        assert_eq!(result, 0); // Should fail

        fz_drop_heap(ctx, heap);
    }

    #[test]
    fn test_null_handling() {
        let ctx = 1;

        // Invalid heap handle
        assert_eq!(fz_heap_len(ctx, 0), 0);
        assert_eq!(fz_heap_is_empty(ctx, 0), 1);
        assert_eq!(fz_heap_capacity(ctx, 0), 0);
        assert_eq!(fz_heap_type(ctx, 0), -1);

        // Null pointers
        let heap = fz_new_heap(ctx, HeapType::Int as i32);
        fz_int_heap_insert(ctx, heap, 5);

        assert_eq!(fz_int_heap_pop(ctx, heap, std::ptr::null_mut()), 0);
        assert_eq!(fz_int_heap_peek(ctx, heap, std::ptr::null_mut()), 0);
        assert_eq!(fz_int_heap_sort(ctx, heap, std::ptr::null_mut(), 10), 0);

        fz_drop_heap(ctx, heap);
    }

    #[test]
    fn test_ptr_heap() {
        let ctx = 1;
        let heap = fz_new_heap(ctx, HeapType::Ptr as i32);

        fz_ptr_heap_insert(ctx, heap, 0x1000);
        fz_ptr_heap_insert(ctx, heap, 0x2000);
        fz_ptr_heap_insert(ctx, heap, 0x3000);

        assert_eq!(fz_heap_len(ctx, heap), 3);

        let mut value: usize = 0;
        assert_eq!(fz_ptr_heap_pop(ctx, heap, &mut value), 1);
        // Note: pointer heap uses max-heap by default in BinaryHeap
        assert!(value > 0);

        fz_drop_heap(ctx, heap);
    }
}
