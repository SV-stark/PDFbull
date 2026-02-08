//! Memory Profiler - Detailed memory leak detection and profiling
//!
//! This module provides comprehensive memory profiling capabilities:
//! - Handle allocation tracking with stack traces
//! - Leak detection for unreleased handles
//! - Memory usage statistics by type
//! - Allocation timeline for debugging
//!
//! Enable with the `profiling` feature flag.

use std::backtrace::Backtrace;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{LazyLock, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime};

use super::Handle;

// ============================================================================
// Configuration
// ============================================================================

/// Whether profiling is enabled at runtime
static PROFILING_ENABLED: AtomicBool = AtomicBool::new(false);

/// Whether to capture stack traces (expensive but useful for debugging)
static CAPTURE_STACK_TRACES: AtomicBool = AtomicBool::new(false);

/// Enable memory profiling
#[unsafe(no_mangle)]
pub extern "C" fn fz_enable_memory_profiling(enabled: i32) {
    PROFILING_ENABLED.store(enabled != 0, Ordering::SeqCst);
}

/// Enable stack trace capture (slower but more detailed)
#[unsafe(no_mangle)]
pub extern "C" fn fz_enable_stack_traces(enabled: i32) {
    CAPTURE_STACK_TRACES.store(enabled != 0, Ordering::SeqCst);
}

/// Check if profiling is enabled
#[inline]
pub fn is_profiling_enabled() -> bool {
    PROFILING_ENABLED.load(Ordering::Relaxed)
}

// ============================================================================
// Allocation Record
// ============================================================================

/// Resource type being tracked
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceType {
    Context = 0,
    Buffer = 1,
    Stream = 2,
    Pixmap = 3,
    Document = 4,
    Page = 5,
    Font = 6,
    Image = 7,
    Path = 8,
    Text = 9,
    Device = 10,
    DisplayList = 11,
    Colorspace = 12,
    PdfObject = 13,
    Outline = 14,
    Link = 15,
    Annotation = 16,
    StextPage = 17,
    Cookie = 18,
    Archive = 19,
    Other = 255,
}

impl ResourceType {
    pub fn name(&self) -> &'static str {
        match self {
            ResourceType::Context => "Context",
            ResourceType::Buffer => "Buffer",
            ResourceType::Stream => "Stream",
            ResourceType::Pixmap => "Pixmap",
            ResourceType::Document => "Document",
            ResourceType::Page => "Page",
            ResourceType::Font => "Font",
            ResourceType::Image => "Image",
            ResourceType::Path => "Path",
            ResourceType::Text => "Text",
            ResourceType::Device => "Device",
            ResourceType::DisplayList => "DisplayList",
            ResourceType::Colorspace => "Colorspace",
            ResourceType::PdfObject => "PdfObject",
            ResourceType::Outline => "Outline",
            ResourceType::Link => "Link",
            ResourceType::Annotation => "Annotation",
            ResourceType::StextPage => "StextPage",
            ResourceType::Cookie => "Cookie",
            ResourceType::Archive => "Archive",
            ResourceType::Other => "Other",
        }
    }
}

/// Record of a single allocation
#[derive(Debug, Clone)]
pub struct AllocationRecord {
    /// Handle ID
    pub handle: Handle,
    /// Type of resource
    pub resource_type: ResourceType,
    /// Estimated size in bytes
    pub size_bytes: usize,
    /// When the allocation occurred
    pub allocated_at: Instant,
    /// System time for logging
    pub timestamp: SystemTime,
    /// Stack trace at allocation (if enabled)
    pub stack_trace: Option<String>,
    /// Thread ID that allocated
    pub thread_id: std::thread::ThreadId,
    /// Thread name (if set)
    pub thread_name: Option<String>,
    /// Custom tag for grouping
    pub tag: Option<String>,
}

impl AllocationRecord {
    pub fn new(handle: Handle, resource_type: ResourceType, size_bytes: usize) -> Self {
        let thread = std::thread::current();
        Self {
            handle,
            resource_type,
            size_bytes,
            allocated_at: Instant::now(),
            timestamp: SystemTime::now(),
            stack_trace: if CAPTURE_STACK_TRACES.load(Ordering::Relaxed) {
                Some(format!("{}", Backtrace::capture()))
            } else {
                None
            },
            thread_id: thread.id(),
            thread_name: thread.name().map(String::from),
            tag: None,
        }
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    pub fn age(&self) -> Duration {
        self.allocated_at.elapsed()
    }
}

// ============================================================================
// Global Profiler State
// ============================================================================

/// Global allocation tracker
pub struct MemoryProfiler {
    /// All active allocations by handle
    allocations: RwLock<HashMap<Handle, AllocationRecord>>,
    /// Historical allocations (for timeline)
    history: Mutex<Vec<AllocationEvent>>,
    /// Statistics by resource type
    stats_by_type: RwLock<HashMap<ResourceType, TypeStats>>,
    /// Global statistics
    global_stats: GlobalStats,
    /// Maximum history entries to keep
    max_history: AtomicUsize,
    /// Start time for relative timestamps
    start_time: Instant,
}

/// Event types for allocation history
#[derive(Debug, Clone)]
pub enum AllocationEvent {
    Allocated {
        handle: Handle,
        resource_type: ResourceType,
        size_bytes: usize,
        timestamp: Instant,
    },
    Deallocated {
        handle: Handle,
        resource_type: ResourceType,
        size_bytes: usize,
        timestamp: Instant,
        lifetime: Duration,
    },
}

/// Statistics for a specific resource type
#[derive(Debug, Clone, Default)]
pub struct TypeStats {
    pub current_count: u64,
    pub current_bytes: u64,
    pub total_allocated: u64,
    pub total_deallocated: u64,
    pub total_bytes_allocated: u64,
    pub total_bytes_deallocated: u64,
    pub peak_count: u64,
    pub peak_bytes: u64,
}

/// Global statistics
#[derive(Debug, Default)]
pub struct GlobalStats {
    pub total_handles_created: AtomicU64,
    pub total_handles_destroyed: AtomicU64,
    pub current_handles: AtomicU64,
    pub current_bytes: AtomicU64,
    pub peak_handles: AtomicU64,
    pub peak_bytes: AtomicU64,
}

impl MemoryProfiler {
    pub fn new() -> Self {
        Self {
            allocations: RwLock::new(HashMap::new()),
            history: Mutex::new(Vec::with_capacity(10000)),
            stats_by_type: RwLock::new(HashMap::new()),
            global_stats: GlobalStats::default(),
            max_history: AtomicUsize::new(100000),
            start_time: Instant::now(),
        }
    }

    /// Record an allocation
    pub fn record_allocation(&self, record: AllocationRecord) {
        if !is_profiling_enabled() {
            return;
        }

        let handle = record.handle;
        let resource_type = record.resource_type;
        let size_bytes = record.size_bytes;

        // Update allocations map
        {
            let mut allocs = self.allocations.write().unwrap();
            allocs.insert(handle, record);
        }

        // Update type-specific stats
        {
            let mut stats = self.stats_by_type.write().unwrap();
            let type_stats = stats.entry(resource_type).or_default();
            type_stats.current_count += 1;
            type_stats.current_bytes += size_bytes as u64;
            type_stats.total_allocated += 1;
            type_stats.total_bytes_allocated += size_bytes as u64;
            type_stats.peak_count = type_stats.peak_count.max(type_stats.current_count);
            type_stats.peak_bytes = type_stats.peak_bytes.max(type_stats.current_bytes);
        }

        // Update global stats
        let current = self
            .global_stats
            .current_handles
            .fetch_add(1, Ordering::Relaxed)
            + 1;
        self.global_stats
            .total_handles_created
            .fetch_add(1, Ordering::Relaxed);
        self.global_stats
            .current_bytes
            .fetch_add(size_bytes as u64, Ordering::Relaxed);

        // Update peaks
        let mut peak = self.global_stats.peak_handles.load(Ordering::Relaxed);
        while current > peak {
            match self.global_stats.peak_handles.compare_exchange_weak(
                peak,
                current,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(p) => peak = p,
            }
        }

        // Add to history
        self.add_history_event(AllocationEvent::Allocated {
            handle,
            resource_type,
            size_bytes,
            timestamp: Instant::now(),
        });
    }

    /// Record a deallocation
    pub fn record_deallocation(&self, handle: Handle) -> Option<AllocationRecord> {
        if !is_profiling_enabled() {
            return None;
        }

        // Remove from allocations
        let record = {
            let mut allocs = self.allocations.write().unwrap();
            allocs.remove(&handle)
        };

        if let Some(ref rec) = record {
            let resource_type = rec.resource_type;
            let size_bytes = rec.size_bytes;
            let lifetime = rec.age();

            // Update type-specific stats
            {
                let mut stats = self.stats_by_type.write().unwrap();
                if let Some(type_stats) = stats.get_mut(&resource_type) {
                    type_stats.current_count = type_stats.current_count.saturating_sub(1);
                    type_stats.current_bytes =
                        type_stats.current_bytes.saturating_sub(size_bytes as u64);
                    type_stats.total_deallocated += 1;
                    type_stats.total_bytes_deallocated += size_bytes as u64;
                }
            }

            // Update global stats
            self.global_stats
                .current_handles
                .fetch_sub(1, Ordering::Relaxed);
            self.global_stats
                .total_handles_destroyed
                .fetch_add(1, Ordering::Relaxed);
            self.global_stats
                .current_bytes
                .fetch_sub(size_bytes as u64, Ordering::Relaxed);

            // Add to history
            self.add_history_event(AllocationEvent::Deallocated {
                handle,
                resource_type,
                size_bytes,
                timestamp: Instant::now(),
                lifetime,
            });
        }

        record
    }

    fn add_history_event(&self, event: AllocationEvent) {
        let max = self.max_history.load(Ordering::Relaxed);
        let mut history = self.history.lock().unwrap();

        // Trim if over limit
        if history.len() >= max {
            let drain_count = max / 10; // Remove 10% when full
            history.drain(0..drain_count);
        }

        history.push(event);
    }

    /// Get all currently live allocations
    pub fn get_live_allocations(&self) -> Vec<AllocationRecord> {
        let allocs = self.allocations.read().unwrap();
        allocs.values().cloned().collect()
    }

    /// Get allocations older than a threshold (potential leaks)
    pub fn get_potential_leaks(&self, min_age: Duration) -> Vec<AllocationRecord> {
        let allocs = self.allocations.read().unwrap();
        allocs
            .values()
            .filter(|r| r.age() >= min_age)
            .cloned()
            .collect()
    }

    /// Get allocations by resource type
    pub fn get_allocations_by_type(&self, resource_type: ResourceType) -> Vec<AllocationRecord> {
        let allocs = self.allocations.read().unwrap();
        allocs
            .values()
            .filter(|r| r.resource_type == resource_type)
            .cloned()
            .collect()
    }

    /// Get statistics by type
    pub fn get_stats_by_type(&self) -> HashMap<ResourceType, TypeStats> {
        self.stats_by_type.read().unwrap().clone()
    }

    /// Get global statistics snapshot
    pub fn get_global_stats(&self) -> GlobalStatsSnapshot {
        GlobalStatsSnapshot {
            total_handles_created: self
                .global_stats
                .total_handles_created
                .load(Ordering::Relaxed),
            total_handles_destroyed: self
                .global_stats
                .total_handles_destroyed
                .load(Ordering::Relaxed),
            current_handles: self.global_stats.current_handles.load(Ordering::Relaxed),
            current_bytes: self.global_stats.current_bytes.load(Ordering::Relaxed),
            peak_handles: self.global_stats.peak_handles.load(Ordering::Relaxed),
            peak_bytes: self.global_stats.peak_bytes.load(Ordering::Relaxed),
            uptime: self.start_time.elapsed(),
        }
    }

    /// Generate a leak report
    pub fn generate_leak_report(&self, min_age: Duration) -> LeakReport {
        let potential_leaks = self.get_potential_leaks(min_age);
        let stats = self.get_global_stats();

        let mut leaks_by_type: HashMap<ResourceType, Vec<AllocationRecord>> = HashMap::new();
        for leak in potential_leaks {
            leaks_by_type
                .entry(leak.resource_type)
                .or_default()
                .push(leak);
        }

        LeakReport {
            generated_at: SystemTime::now(),
            min_age_threshold: min_age,
            total_potential_leaks: leaks_by_type.values().map(|v| v.len()).sum(),
            leaks_by_type,
            global_stats: stats,
        }
    }

    /// Reset all profiling data
    pub fn reset(&self) {
        self.allocations.write().unwrap().clear();
        self.history.lock().unwrap().clear();
        self.stats_by_type.write().unwrap().clear();
        self.global_stats
            .total_handles_created
            .store(0, Ordering::Relaxed);
        self.global_stats
            .total_handles_destroyed
            .store(0, Ordering::Relaxed);
        self.global_stats
            .current_handles
            .store(0, Ordering::Relaxed);
        self.global_stats.current_bytes.store(0, Ordering::Relaxed);
        self.global_stats.peak_handles.store(0, Ordering::Relaxed);
        self.global_stats.peak_bytes.store(0, Ordering::Relaxed);
    }
}

impl Default for MemoryProfiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of global statistics
#[derive(Debug, Clone)]
pub struct GlobalStatsSnapshot {
    pub total_handles_created: u64,
    pub total_handles_destroyed: u64,
    pub current_handles: u64,
    pub current_bytes: u64,
    pub peak_handles: u64,
    pub peak_bytes: u64,
    pub uptime: Duration,
}

/// Leak detection report
#[derive(Debug, Clone)]
pub struct LeakReport {
    pub generated_at: SystemTime,
    pub min_age_threshold: Duration,
    pub total_potential_leaks: usize,
    pub leaks_by_type: HashMap<ResourceType, Vec<AllocationRecord>>,
    pub global_stats: GlobalStatsSnapshot,
}

impl LeakReport {
    pub fn to_string_report(&self) -> String {
        let mut report = String::new();
        report.push_str("=== MicroPDF Memory Leak Report ===\n\n");

        report.push_str(&format!(
            "Generated: {:?}\n",
            self.generated_at
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        ));
        report.push_str(&format!(
            "Min age threshold: {:?}\n",
            self.min_age_threshold
        ));
        report.push_str(&format!(
            "Total potential leaks: {}\n\n",
            self.total_potential_leaks
        ));

        report.push_str("--- Global Statistics ---\n");
        report.push_str(&format!(
            "Handles created: {}\n",
            self.global_stats.total_handles_created
        ));
        report.push_str(&format!(
            "Handles destroyed: {}\n",
            self.global_stats.total_handles_destroyed
        ));
        report.push_str(&format!(
            "Current handles: {}\n",
            self.global_stats.current_handles
        ));
        report.push_str(&format!(
            "Current memory: {} bytes\n",
            self.global_stats.current_bytes
        ));
        report.push_str(&format!(
            "Peak handles: {}\n",
            self.global_stats.peak_handles
        ));
        report.push_str(&format!(
            "Peak memory: {} bytes\n",
            self.global_stats.peak_bytes
        ));
        report.push_str(&format!("Uptime: {:?}\n\n", self.global_stats.uptime));

        report.push_str("--- Leaks by Type ---\n");
        for (resource_type, leaks) in &self.leaks_by_type {
            if !leaks.is_empty() {
                report.push_str(&format!(
                    "\n{} ({} leaks):\n",
                    resource_type.name(),
                    leaks.len()
                ));
                for (i, leak) in leaks.iter().take(10).enumerate() {
                    report.push_str(&format!(
                        "  {}. Handle {} - {} bytes, age {:?}",
                        i + 1,
                        leak.handle,
                        leak.size_bytes,
                        leak.age()
                    ));
                    if let Some(ref tag) = leak.tag {
                        report.push_str(&format!(", tag: {}", tag));
                    }
                    report.push('\n');
                    if let Some(ref trace) = leak.stack_trace {
                        // Only show first few lines of stack trace
                        for line in trace.lines().take(5) {
                            report.push_str(&format!("      {}\n", line));
                        }
                    }
                }
                if leaks.len() > 10 {
                    report.push_str(&format!("  ... and {} more\n", leaks.len() - 10));
                }
            }
        }

        report
    }
}

// ============================================================================
// Global Instance
// ============================================================================

/// Global memory profiler instance
pub static MEMORY_PROFILER: LazyLock<MemoryProfiler> = LazyLock::new(MemoryProfiler::new);

// ============================================================================
// FFI Functions
// ============================================================================

/// Get the number of currently live handles
#[unsafe(no_mangle)]
pub extern "C" fn fz_profiler_live_handle_count() -> u64 {
    MEMORY_PROFILER
        .global_stats
        .current_handles
        .load(Ordering::Relaxed)
}

/// Get the current memory usage in bytes
#[unsafe(no_mangle)]
pub extern "C" fn fz_profiler_current_bytes() -> u64 {
    MEMORY_PROFILER
        .global_stats
        .current_bytes
        .load(Ordering::Relaxed)
}

/// Get peak handle count
#[unsafe(no_mangle)]
pub extern "C" fn fz_profiler_peak_handles() -> u64 {
    MEMORY_PROFILER
        .global_stats
        .peak_handles
        .load(Ordering::Relaxed)
}

/// Get peak memory usage
#[unsafe(no_mangle)]
pub extern "C" fn fz_profiler_peak_bytes() -> u64 {
    MEMORY_PROFILER
        .global_stats
        .peak_bytes
        .load(Ordering::Relaxed)
}

/// Get count of potential leaks (handles older than given seconds)
#[unsafe(no_mangle)]
pub extern "C" fn fz_profiler_potential_leak_count(min_age_seconds: u64) -> u64 {
    let min_age = Duration::from_secs(min_age_seconds);
    MEMORY_PROFILER.get_potential_leaks(min_age).len() as u64
}

/// Get count of handles by type
#[unsafe(no_mangle)]
pub extern "C" fn fz_profiler_handle_count_by_type(resource_type: ResourceType) -> u64 {
    let stats = MEMORY_PROFILER.stats_by_type.read().unwrap();
    stats
        .get(&resource_type)
        .map(|s| s.current_count)
        .unwrap_or(0)
}

/// Reset profiler data
#[unsafe(no_mangle)]
pub extern "C" fn fz_profiler_reset() {
    MEMORY_PROFILER.reset();
}

/// Print leak report to stderr
#[unsafe(no_mangle)]
pub extern "C" fn fz_profiler_print_leak_report(min_age_seconds: u64) {
    let min_age = Duration::from_secs(min_age_seconds);
    let report = MEMORY_PROFILER.generate_leak_report(min_age);
    eprintln!("{}", report.to_string_report());
}

// ============================================================================
// Helper Functions for Integration
// ============================================================================

/// Record an allocation (call from HandleStore.insert)
pub fn track_allocation(handle: Handle, resource_type: ResourceType, size_bytes: usize) {
    if is_profiling_enabled() {
        let record = AllocationRecord::new(handle, resource_type, size_bytes);
        MEMORY_PROFILER.record_allocation(record);
    }
}

/// Record an allocation with a tag
pub fn track_allocation_tagged(
    handle: Handle,
    resource_type: ResourceType,
    size_bytes: usize,
    tag: &str,
) {
    if is_profiling_enabled() {
        let record = AllocationRecord::new(handle, resource_type, size_bytes).with_tag(tag);
        MEMORY_PROFILER.record_allocation(record);
    }
}

/// Record a deallocation (call from HandleStore.remove)
pub fn track_deallocation(handle: Handle) {
    if is_profiling_enabled() {
        MEMORY_PROFILER.record_deallocation(handle);
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_allocation_tracking() {
        // Enable profiling for this test
        fz_enable_memory_profiling(1);

        // Use our own local profiler instance
        let profiler = MemoryProfiler::new();

        // Record some allocations
        let record1 = AllocationRecord::new(1, ResourceType::Buffer, 1024);
        profiler.record_allocation(record1);

        let record2 = AllocationRecord::new(2, ResourceType::Pixmap, 4096);
        profiler.record_allocation(record2);

        // Check local profiler stats
        let stats = profiler.get_global_stats();
        assert_eq!(stats.current_handles, 2);
        assert_eq!(stats.current_bytes, 5120);

        // Deallocate one
        profiler.record_deallocation(1);

        let stats = profiler.get_global_stats();
        assert_eq!(stats.current_handles, 1);
        assert_eq!(stats.current_bytes, 4096);

        fz_enable_memory_profiling(0);
    }

    #[test]
    #[serial]
    fn test_leak_detection() {
        // Enable profiling for this test
        fz_enable_memory_profiling(1);

        // Use our own local profiler instance
        let profiler = MemoryProfiler::new();

        // Record allocations
        for i in 0..5 {
            let record = AllocationRecord::new(i, ResourceType::Buffer, 100);
            profiler.record_allocation(record);
        }

        // Get potential leaks (immediately, so min_age = 0)
        let leaks = profiler.get_potential_leaks(Duration::ZERO);
        assert_eq!(leaks.len(), 5);

        // Generate report
        let report = profiler.generate_leak_report(Duration::ZERO);
        assert_eq!(report.total_potential_leaks, 5);

        fz_enable_memory_profiling(0);
    }

    #[test]
    #[serial]
    fn test_type_stats() {
        // Enable profiling for this test
        fz_enable_memory_profiling(1);

        // Use our own local profiler instance
        let profiler = MemoryProfiler::new();

        // Record allocations of different types
        profiler.record_allocation(AllocationRecord::new(1, ResourceType::Buffer, 100));
        profiler.record_allocation(AllocationRecord::new(2, ResourceType::Buffer, 200));
        profiler.record_allocation(AllocationRecord::new(3, ResourceType::Pixmap, 1000));

        let stats = profiler.get_stats_by_type();

        let buffer_stats = stats.get(&ResourceType::Buffer).unwrap();
        assert_eq!(buffer_stats.current_count, 2);
        assert_eq!(buffer_stats.current_bytes, 300);

        let pixmap_stats = stats.get(&ResourceType::Pixmap).unwrap();
        assert_eq!(pixmap_stats.current_count, 1);
        assert_eq!(pixmap_stats.current_bytes, 1000);

        fz_enable_memory_profiling(0);
    }
}
