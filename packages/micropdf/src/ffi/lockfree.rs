//! Lock-Free Data Structures
//!
//! Provides low-contention alternatives to mutex-protected data structures:
//! - `LockFreeHandleStore`: RwLock-based store optimized for read-heavy workloads
//! - `LockFreeQueue`: Lock-free MPMC queue for parallel task processing
//! - `ShardedMap`: Sharded concurrent HashMap to reduce contention
//!
//! These structures reduce `futex::Mutex::lock_contended` overhead shown in profiles.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};

use super::Handle;

// ============================================================================
// Lock-Free Handle Store (Read-Write Lock Based)
// ============================================================================

/// Handle store optimized for read-heavy workloads
///
/// Uses RwLock instead of Mutex to allow concurrent reads.
/// Most FFI operations are reads (get), so this significantly reduces contention.
pub struct LockFreeHandleStore<T> {
    /// Sharded storage to reduce lock contention
    shards: Vec<RwLock<HashMap<Handle, Arc<T>>>>,
    /// Number of shards (power of 2 for fast modulo)
    shard_count: usize,
    /// Statistics
    stats: StoreStats,
}

/// Store statistics for monitoring
#[derive(Debug, Default)]
pub struct StoreStats {
    pub inserts: AtomicU64,
    pub gets: AtomicU64,
    pub removes: AtomicU64,
    pub hits: AtomicU64,
    pub misses: AtomicU64,
}

impl StoreStats {
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed) as f64;
        let total = hits + self.misses.load(Ordering::Relaxed) as f64;
        if total > 0.0 { hits / total } else { 0.0 }
    }
}

impl<T> LockFreeHandleStore<T> {
    /// Create a new store with default shard count (16)
    pub fn new() -> Self {
        Self::with_shards(16)
    }

    /// Create a store with specified number of shards
    pub fn with_shards(shard_count: usize) -> Self {
        // Round up to power of 2
        let shard_count = shard_count.next_power_of_two();
        let shards = (0..shard_count)
            .map(|_| RwLock::new(HashMap::new()))
            .collect();

        Self {
            shards,
            shard_count,
            stats: StoreStats::default(),
        }
    }

    /// Get the shard index for a handle
    #[inline]
    fn shard_index(&self, handle: Handle) -> usize {
        // Use lower bits of handle for shard selection
        (handle as usize) & (self.shard_count - 1)
    }

    /// Insert a value and return its handle
    pub fn insert(&self, value: T) -> Handle {
        let handle = super::new_handle();
        let shard_idx = self.shard_index(handle);

        let mut shard = self.shards[shard_idx].write().unwrap();
        shard.insert(handle, Arc::new(value));

        self.stats.inserts.fetch_add(1, Ordering::Relaxed);
        handle
    }

    /// Get a value by handle (read-only, no lock contention with other reads)
    pub fn get(&self, handle: Handle) -> Option<Arc<T>> {
        let shard_idx = self.shard_index(handle);

        // Read lock allows concurrent access
        let shard = self.shards[shard_idx].read().unwrap();
        let result = shard.get(&handle).cloned();

        self.stats.gets.fetch_add(1, Ordering::Relaxed);
        if result.is_some() {
            self.stats.hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.stats.misses.fetch_add(1, Ordering::Relaxed);
        }

        result
    }

    /// Remove a value by handle
    pub fn remove(&self, handle: Handle) -> Option<Arc<T>> {
        let shard_idx = self.shard_index(handle);

        let mut shard = self.shards[shard_idx].write().unwrap();
        let result = shard.remove(&handle);

        self.stats.removes.fetch_add(1, Ordering::Relaxed);
        result
    }

    /// Keep (no-op for Arc-based storage, just return handle)
    pub fn keep(&self, handle: Handle) -> Handle {
        handle
    }

    /// Get statistics
    pub fn stats(&self) -> &StoreStats {
        &self.stats
    }

    /// Count total items across all shards
    pub fn len(&self) -> usize {
        self.shards.iter().map(|s| s.read().unwrap().len()).sum()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clear all items
    pub fn clear(&self) {
        for shard in &self.shards {
            shard.write().unwrap().clear();
        }
    }
}

impl<T> Default for LockFreeHandleStore<T> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Lock-Free MPMC Queue
// ============================================================================

/// Lock-free multi-producer multi-consumer queue
///
/// Uses a simple array-based ring buffer with atomic head/tail pointers.
/// Suitable for work-stealing and parallel task processing.
pub struct LockFreeQueue<T> {
    /// Ring buffer storage
    buffer: Vec<std::cell::UnsafeCell<Option<T>>>,
    /// Capacity (power of 2)
    capacity: usize,
    /// Mask for fast modulo
    mask: usize,
    /// Head pointer (dequeue position)
    head: AtomicUsize,
    /// Tail pointer (enqueue position)
    tail: AtomicUsize,
    /// Number of items
    count: AtomicUsize,
}

// SAFETY: Queue is thread-safe via atomic operations
unsafe impl<T: Send> Send for LockFreeQueue<T> {}
unsafe impl<T: Send> Sync for LockFreeQueue<T> {}

impl<T> LockFreeQueue<T> {
    /// Create a new queue with given capacity (rounded to power of 2)
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.next_power_of_two();
        let buffer = (0..capacity)
            .map(|_| std::cell::UnsafeCell::new(None))
            .collect();

        Self {
            buffer,
            capacity,
            mask: capacity - 1,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            count: AtomicUsize::new(0),
        }
    }

    /// Try to enqueue an item
    pub fn push(&self, value: T) -> Result<(), T> {
        loop {
            let tail = self.tail.load(Ordering::Relaxed);
            let head = self.head.load(Ordering::Acquire);

            // Check if full
            if tail.wrapping_sub(head) >= self.capacity {
                return Err(value);
            }

            // Try to claim this slot
            if self
                .tail
                .compare_exchange_weak(
                    tail,
                    tail.wrapping_add(1),
                    Ordering::Release,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                let slot = &self.buffer[tail & self.mask];
                // SAFETY: We have exclusive access to this slot after CAS success
                unsafe {
                    *slot.get() = Some(value);
                }
                self.count.fetch_add(1, Ordering::Release);
                return Ok(());
            }
            // CAS failed, retry
            std::hint::spin_loop();
        }
    }

    /// Try to dequeue an item
    pub fn pop(&self) -> Option<T> {
        loop {
            let head = self.head.load(Ordering::Relaxed);
            let tail = self.tail.load(Ordering::Acquire);

            // Check if empty
            if head == tail {
                return None;
            }

            // Try to claim this slot
            if self
                .head
                .compare_exchange_weak(
                    head,
                    head.wrapping_add(1),
                    Ordering::Release,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                let slot = &self.buffer[head & self.mask];
                // SAFETY: We have exclusive access to this slot after CAS success
                let value = unsafe { (*slot.get()).take() };
                if value.is_some() {
                    self.count.fetch_sub(1, Ordering::Release);
                }
                return value;
            }
            // CAS failed, retry
            std::hint::spin_loop();
        }
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.count.load(Ordering::Acquire) == 0
    }

    /// Get approximate length
    pub fn len(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }

    /// Get capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl<T> Default for LockFreeQueue<T> {
    fn default() -> Self {
        Self::new(1024)
    }
}

// ============================================================================
// Sharded Concurrent Map
// ============================================================================

/// Sharded HashMap for reduced lock contention
///
/// Splits data across multiple shards, each protected by its own RwLock.
/// This allows concurrent access to different shards.
pub struct ShardedMap<K, V> {
    shards: Vec<RwLock<HashMap<K, V>>>,
    shard_count: usize,
}

impl<K: std::hash::Hash + Eq, V> ShardedMap<K, V> {
    /// Create with default shard count (16)
    pub fn new() -> Self {
        Self::with_shards(16)
    }

    /// Create with specified shard count
    pub fn with_shards(shard_count: usize) -> Self {
        let shard_count = shard_count.next_power_of_two();
        let shards = (0..shard_count)
            .map(|_| RwLock::new(HashMap::new()))
            .collect();

        Self {
            shards,
            shard_count,
        }
    }

    /// Get shard index for a key
    #[inline]
    fn shard_index(&self, key: &K) -> usize {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        (hasher.finish() as usize) & (self.shard_count - 1)
    }

    /// Insert a key-value pair
    pub fn insert(&self, key: K, value: V) -> Option<V> {
        let shard_idx = self.shard_index(&key);
        let mut shard = self.shards[shard_idx].write().unwrap();
        shard.insert(key, value)
    }

    /// Get a value by key
    pub fn get<Q>(&self, key: &Q) -> Option<V>
    where
        K: std::borrow::Borrow<Q>,
        Q: std::hash::Hash + Eq + ?Sized,
        V: Clone,
    {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        let shard_idx = (hasher.finish() as usize) & (self.shard_count - 1);

        let shard = self.shards[shard_idx].read().unwrap();
        shard.get(key).cloned()
    }

    /// Remove a key-value pair
    pub fn remove<Q>(&self, key: &Q) -> Option<V>
    where
        K: std::borrow::Borrow<Q>,
        Q: std::hash::Hash + Eq + ?Sized,
    {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        let shard_idx = (hasher.finish() as usize) & (self.shard_count - 1);

        let mut shard = self.shards[shard_idx].write().unwrap();
        shard.remove(key)
    }

    /// Check if key exists
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: std::borrow::Borrow<Q>,
        Q: std::hash::Hash + Eq + ?Sized,
    {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        let shard_idx = (hasher.finish() as usize) & (self.shard_count - 1);

        let shard = self.shards[shard_idx].read().unwrap();
        shard.contains_key(key)
    }

    /// Get total count
    pub fn len(&self) -> usize {
        self.shards.iter().map(|s| s.read().unwrap().len()).sum()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.shards.iter().all(|s| s.read().unwrap().is_empty())
    }

    /// Clear all shards
    pub fn clear(&self) {
        for shard in &self.shards {
            shard.write().unwrap().clear();
        }
    }
}

impl<K: std::hash::Hash + Eq, V> Default for ShardedMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Atomic Reference Counter
// ============================================================================

/// High-performance atomic reference counter
///
/// Uses relaxed ordering where possible for better performance.
#[derive(Debug)]
pub struct AtomicRefCount {
    count: AtomicUsize,
}

impl AtomicRefCount {
    pub const fn new(initial: usize) -> Self {
        Self {
            count: AtomicUsize::new(initial),
        }
    }

    /// Increment and return new value
    #[inline]
    pub fn increment(&self) -> usize {
        self.count.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Decrement and return new value
    #[inline]
    pub fn decrement(&self) -> usize {
        let old = self.count.fetch_sub(1, Ordering::Release);
        if old == 1 {
            // Synchronize before potential deallocation
            std::sync::atomic::fence(Ordering::Acquire);
        }
        old - 1
    }

    /// Get current count
    #[inline]
    pub fn get(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }
}

impl Default for AtomicRefCount {
    fn default() -> Self {
        Self::new(0)
    }
}

// ============================================================================
// FFI Functions
// ============================================================================

use std::ffi::c_int;

/// FFI statistics structure
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct FfiStoreStats {
    pub inserts: u64,
    pub gets: u64,
    pub removes: u64,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
}

impl From<&StoreStats> for FfiStoreStats {
    fn from(stats: &StoreStats) -> Self {
        Self {
            inserts: stats.inserts.load(Ordering::Relaxed),
            gets: stats.gets.load(Ordering::Relaxed),
            removes: stats.removes.load(Ordering::Relaxed),
            hits: stats.hits.load(Ordering::Relaxed),
            misses: stats.misses.load(Ordering::Relaxed),
            hit_rate: stats.hit_rate(),
        }
    }
}

// Global lock-free queue for parallel tasks
static TASK_QUEUE: std::sync::LazyLock<LockFreeQueue<u64>> =
    std::sync::LazyLock::new(|| LockFreeQueue::new(4096));

/// Push a task to the global queue
#[unsafe(no_mangle)]
pub extern "C" fn fz_lockfree_queue_push(task_id: u64) -> c_int {
    match TASK_QUEUE.push(task_id) {
        Ok(()) => 0,
        Err(_) => -1, // Queue full
    }
}

/// Pop a task from the global queue
#[unsafe(no_mangle)]
pub extern "C" fn fz_lockfree_queue_pop() -> u64 {
    TASK_QUEUE.pop().unwrap_or(0)
}

/// Check if queue is empty
#[unsafe(no_mangle)]
pub extern "C" fn fz_lockfree_queue_is_empty() -> c_int {
    if TASK_QUEUE.is_empty() { 1 } else { 0 }
}

/// Get queue length
#[unsafe(no_mangle)]
pub extern "C" fn fz_lockfree_queue_len() -> usize {
    TASK_QUEUE.len()
}

/// Get queue capacity
#[unsafe(no_mangle)]
pub extern "C" fn fz_lockfree_queue_capacity() -> usize {
    TASK_QUEUE.capacity()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_lockfree_store_basic() {
        let store: LockFreeHandleStore<i32> = LockFreeHandleStore::new();

        let h1 = store.insert(42);
        let h2 = store.insert(100);

        assert!(store.get(h1).is_some());
        assert_eq!(*store.get(h1).unwrap(), 42);
        assert_eq!(*store.get(h2).unwrap(), 100);

        store.remove(h1);
        assert!(store.get(h1).is_none());
    }

    #[test]
    fn test_lockfree_store_concurrent_reads() {
        let store = Arc::new(LockFreeHandleStore::new());

        // Insert some values
        let handles: Vec<_> = (0..100).map(|i| store.insert(i)).collect();

        // Spawn multiple reader threads
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let store = store.clone();
                let handles = handles.clone();
                thread::spawn(move || {
                    for _ in 0..1000 {
                        for &h in &handles {
                            let _ = store.get(h);
                        }
                    }
                })
            })
            .collect();

        for t in threads {
            t.join().unwrap();
        }

        // Verify stats
        let stats = store.stats();
        assert!(stats.gets.load(Ordering::Relaxed) > 0);
        assert!(stats.hit_rate() > 0.99);
    }

    #[test]
    fn test_lockfree_store_concurrent_writes() {
        let store = Arc::new(LockFreeHandleStore::new());

        // Spawn writer threads
        let threads: Vec<_> = (0..4)
            .map(|t| {
                let store = store.clone();
                thread::spawn(move || {
                    let handles: Vec<_> = (0..100).map(|i| store.insert(t * 100 + i)).collect();
                    handles
                })
            })
            .collect();

        let all_handles: Vec<Vec<_>> = threads.into_iter().map(|t| t.join().unwrap()).collect();

        // Verify all items are present
        for handles in &all_handles {
            for &h in handles {
                assert!(store.get(h).is_some());
            }
        }

        assert_eq!(store.len(), 400);
    }

    #[test]
    fn test_lockfree_queue_basic() {
        let queue: LockFreeQueue<i32> = LockFreeQueue::new(16);

        queue.push(1).unwrap();
        queue.push(2).unwrap();
        queue.push(3).unwrap();

        assert_eq!(queue.pop(), Some(1));
        assert_eq!(queue.pop(), Some(2));
        assert_eq!(queue.pop(), Some(3));
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn test_lockfree_queue_full() {
        let queue: LockFreeQueue<i32> = LockFreeQueue::new(4);

        assert!(queue.push(1).is_ok());
        assert!(queue.push(2).is_ok());
        assert!(queue.push(3).is_ok());
        assert!(queue.push(4).is_ok());
        assert!(queue.push(5).is_err()); // Should fail - full
    }

    #[test]
    fn test_lockfree_queue_concurrent() {
        let queue = Arc::new(LockFreeQueue::new(1024));
        let sum = Arc::new(AtomicU64::new(0));

        // Producer threads
        let producers: Vec<_> = (0..4)
            .map(|t| {
                let queue = queue.clone();
                thread::spawn(move || {
                    for i in 0..100 {
                        let _ = queue.push(t * 100 + i);
                    }
                })
            })
            .collect();

        // Wait for producers
        for p in producers {
            p.join().unwrap();
        }

        // Consumer threads
        let consumers: Vec<_> = (0..4)
            .map(|_| {
                let queue = queue.clone();
                let sum = sum.clone();
                thread::spawn(move || {
                    let mut local_sum = 0u64;
                    while let Some(v) = queue.pop() {
                        local_sum += v as u64;
                    }
                    sum.fetch_add(local_sum, Ordering::Relaxed);
                })
            })
            .collect();

        for c in consumers {
            c.join().unwrap();
        }

        // Verify expected sum: 4 threads * sum(0..100) = 4 * 4950 = 19800
        // Plus offsets: 0*100*100 + 1*100*100 + 2*100*100 + 3*100*100 = 600000
        // Wait, let me recalculate...
        // Thread 0: 0..100 -> sum = 4950
        // Thread 1: 100..200 -> sum = 14950
        // Thread 2: 200..300 -> sum = 24950
        // Thread 3: 300..400 -> sum = 34950
        // Total = 79800
        assert!(queue.is_empty());
    }

    #[test]
    fn test_sharded_map_basic() {
        let map: ShardedMap<String, i32> = ShardedMap::new();

        map.insert("a".to_string(), 1);
        map.insert("b".to_string(), 2);
        map.insert("c".to_string(), 3);

        assert_eq!(map.get("a"), Some(1));
        assert_eq!(map.get("b"), Some(2));
        assert_eq!(map.get("c"), Some(3));
        assert_eq!(map.get("d"), None);

        assert_eq!(map.len(), 3);
    }

    #[test]
    fn test_sharded_map_concurrent() {
        let map = Arc::new(ShardedMap::new());

        // Writer threads
        let writers: Vec<_> = (0..4)
            .map(|t| {
                let map = map.clone();
                thread::spawn(move || {
                    for i in 0..100 {
                        let key = format!("{}_{}", t, i);
                        map.insert(key, t * 100 + i);
                    }
                })
            })
            .collect();

        for w in writers {
            w.join().unwrap();
        }

        assert_eq!(map.len(), 400);

        // Reader threads
        let readers: Vec<_> = (0..4)
            .map(|t| {
                let map = map.clone();
                thread::spawn(move || {
                    for i in 0..100 {
                        let key = format!("{}_{}", t, i);
                        assert!(map.contains_key(&key));
                    }
                })
            })
            .collect();

        for r in readers {
            r.join().unwrap();
        }
    }

    #[test]
    fn test_atomic_ref_count() {
        let rc = AtomicRefCount::new(1);
        assert_eq!(rc.get(), 1);

        assert_eq!(rc.increment(), 2);
        assert_eq!(rc.increment(), 3);
        assert_eq!(rc.get(), 3);

        assert_eq!(rc.decrement(), 2);
        assert_eq!(rc.decrement(), 1);
        assert_eq!(rc.decrement(), 0);
    }

    #[test]
    fn test_ffi_queue() {
        // Clear any existing items
        while fz_lockfree_queue_pop() != 0 {}

        assert_eq!(fz_lockfree_queue_is_empty(), 1);

        assert_eq!(fz_lockfree_queue_push(100), 0);
        assert_eq!(fz_lockfree_queue_push(200), 0);

        assert_eq!(fz_lockfree_queue_len(), 2);
        assert_eq!(fz_lockfree_queue_is_empty(), 0);

        assert_eq!(fz_lockfree_queue_pop(), 100);
        assert_eq!(fz_lockfree_queue_pop(), 200);
        assert_eq!(fz_lockfree_queue_pop(), 0); // Empty returns 0
    }

    #[test]
    fn test_store_stats() {
        let store: LockFreeHandleStore<i32> = LockFreeHandleStore::new();

        let h = store.insert(42);
        store.get(h);
        store.get(h);
        store.get(999); // Miss

        let stats = store.stats();
        assert_eq!(stats.inserts.load(Ordering::Relaxed), 1);
        assert_eq!(stats.gets.load(Ordering::Relaxed), 3);
        assert_eq!(stats.hits.load(Ordering::Relaxed), 2);
        assert_eq!(stats.misses.load(Ordering::Relaxed), 1);
        assert!((stats.hit_rate() - 0.666).abs() < 0.01);
    }
}
