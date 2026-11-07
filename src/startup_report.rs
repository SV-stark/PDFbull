//! A built-in stopwatch for startup: how long PDFbull takes to put a window on
//! screen, open a document and paint its first page, and how much memory and
//! CPU that cost.
//!
//! Every milestone is written to the log (`startup: <milestone> at N ms`).
//! Launching with `--startup-report` additionally prints a short report once
//! the first page is on screen and then exits, so a launch can be timed from a
//! script and compared with another reader on the same machine and file:
//!
//! ```text
//! pdfbull --startup-report big.pdf
//! ```
//!
//! Times are measured from the moment Windows created the process (not from
//! `main`), so they include loading the executable and its DLLs — the same
//! "double-click to page" interval a person experiences.

use std::sync::Mutex;
use std::time::Duration;

/// The command-line switch that turns the report on.
pub const REPORT_FLAG: &str = "--startup-report";

static MARKS: Mutex<Vec<(&'static str, Duration)>> = Mutex::new(Vec::new());
static FALLBACK_START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

/// Call first thing in `main`: anchors the fallback clock and records `main`.
pub fn begin() {
    let _ = FALLBACK_START.set(std::time::Instant::now());
    mark("main entered");
}

/// Was the program started with [`REPORT_FLAG`]?
pub fn requested() -> bool {
    static REQUESTED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *REQUESTED.get_or_init(|| std::env::args().any(|a| a == REPORT_FLAG))
}

/// Time since the process was created.
pub fn elapsed() -> Duration {
    #[cfg(windows)]
    if let Some(d) = windows_since_creation() {
        return d;
    }
    FALLBACK_START
        .get()
        .map(std::time::Instant::elapsed)
        .unwrap_or_default()
}

/// Record `name` the first time it is reached; later calls are ignored.
/// Returns `true` when this call recorded it.
pub fn mark(name: &'static str) -> bool {
    let at = elapsed();
    let Ok(mut marks) = MARKS.lock() else {
        return false;
    };
    if marks.iter().any(|(n, _)| *n == name) {
        return false;
    }
    marks.push((name, at));
    tracing::info!("startup: {name} at {:.1} ms", at.as_secs_f64() * 1000.0);
    true
}

/// Has `name` been recorded yet?
pub fn reached(name: &str) -> bool {
    MARKS
        .lock()
        .map(|m| m.iter().any(|(n, _)| *n == name))
        .unwrap_or(false)
}

/// Memory and CPU use of this process, where the platform reports them.
#[derive(Debug, Default, Clone, Copy)]
pub struct ResourceUse {
    pub working_set_bytes: Option<u64>,
    pub peak_working_set_bytes: Option<u64>,
    pub private_bytes: Option<u64>,
    pub cpu_time: Option<Duration>,
}

pub fn resource_use() -> ResourceUse {
    #[cfg(windows)]
    {
        windows_resource_use()
    }
    #[cfg(target_os = "linux")]
    {
        linux_resource_use()
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        ResourceUse::default()
    }
}

/// The report text: every milestone, then memory and CPU.
pub fn report() -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "PDFbull {} startup report", env!("CARGO_PKG_VERSION"));
    if let Ok(marks) = MARKS.lock() {
        for (name, at) in marks.iter() {
            let _ = writeln!(out, "  {:<22} {:>9.1} ms", name, at.as_secs_f64() * 1000.0);
        }
    }
    let mb = |b: u64| b as f64 / (1024.0 * 1024.0);
    let r = resource_use();
    if let Some(b) = r.working_set_bytes {
        let _ = writeln!(out, "  {:<22} {:>9.1} MB", "memory now", mb(b));
    }
    if let Some(b) = r.peak_working_set_bytes {
        let _ = writeln!(out, "  {:<22} {:>9.1} MB", "memory peak", mb(b));
    }
    if let Some(b) = r.private_bytes {
        let _ = writeln!(out, "  {:<22} {:>9.1} MB", "private memory", mb(b));
    }
    if let Some(t) = r.cpu_time {
        let _ = writeln!(
            out,
            "  {:<22} {:>9.1} ms",
            "CPU time used",
            t.as_secs_f64() * 1000.0
        );
    }
    out
}

/// Print the report (to the console that started us, if any), save it next
/// to the log as `startup_report.txt`, and log it.
pub fn emit() -> String {
    let text = report();
    #[cfg(windows)]
    unsafe {
        use windows::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }
    println!("{text}");
    let path = crate::storage::get_config_dir().join("startup_report.txt");
    let _ = std::fs::write(&path, &text);
    tracing::info!("{text}");
    text
}

#[cfg(windows)]
fn filetime_to_u64(ft: windows::Win32::Foundation::FILETIME) -> u64 {
    (u64::from(ft.dwHighDateTime) << 32) | u64::from(ft.dwLowDateTime)
}

#[cfg(windows)]
fn process_times() -> Option<(u64, u64, u64)> {
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
    let (mut created, mut exited, mut kernel, mut user) = (
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
    );
    // SAFETY: the pseudo-handle is always valid; the out-pointers are live locals.
    unsafe {
        GetProcessTimes(
            GetCurrentProcess(),
            &mut created,
            &mut exited,
            &mut kernel,
            &mut user,
        )
        .ok()?;
    }
    Some((
        filetime_to_u64(created),
        filetime_to_u64(kernel),
        filetime_to_u64(user),
    ))
}

#[cfg(windows)]
fn windows_since_creation() -> Option<Duration> {
    // FILETIME counts 100 ns ticks since 1601-01-01; the Unix epoch is
    // 11 644 473 600 s later.
    const EPOCH_DIFF_TICKS: u64 = 11_644_473_600 * 10_000_000;
    let (created, _, _) = process_times()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?;
    let now_ticks = EPOCH_DIFF_TICKS + (now.as_nanos() / 100) as u64;
    Some(Duration::from_nanos(now_ticks.checked_sub(created)? * 100))
}

#[cfg(windows)]
fn windows_resource_use() -> ResourceUse {
    use windows::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX,
    };
    use windows::Win32::System::Threading::GetCurrentProcess;
    let mut out = ResourceUse::default();
    let mut counters = PROCESS_MEMORY_COUNTERS_EX {
        cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
        ..Default::default()
    };
    // SAFETY: `counters` is a live, correctly sized PROCESS_MEMORY_COUNTERS_EX,
    // which begins with the PROCESS_MEMORY_COUNTERS layout the API expects.
    let ok = unsafe {
        GetProcessMemoryInfo(
            GetCurrentProcess(),
            (&raw mut counters).cast::<PROCESS_MEMORY_COUNTERS>(),
            counters.cb,
        )
        .is_ok()
    };
    if ok {
        out.working_set_bytes = Some(counters.WorkingSetSize as u64);
        out.peak_working_set_bytes = Some(counters.PeakWorkingSetSize as u64);
        out.private_bytes = Some(counters.PrivateUsage as u64);
    }
    if let Some((_, kernel, user)) = process_times() {
        out.cpu_time = Some(Duration::from_nanos((kernel + user) * 100));
    }
    out
}

#[cfg(target_os = "linux")]
fn linux_resource_use() -> ResourceUse {
    let mut out = ResourceUse::default();
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        let kb = |key: &str| -> Option<u64> {
            status
                .lines()
                .find(|l| l.starts_with(key))?
                .split_whitespace()
                .nth(1)?
                .parse::<u64>()
                .ok()
                .map(|v| v * 1024)
        };
        out.working_set_bytes = kb("VmRSS:");
        out.peak_working_set_bytes = kb("VmHWM:");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{mark, reached, report};

    #[test]
    fn a_milestone_is_recorded_once() {
        assert!(mark("test milestone"));
        assert!(!mark("test milestone"));
        assert!(reached("test milestone"));
        assert!(report().contains("test milestone"));
    }
}
