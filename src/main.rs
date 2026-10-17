#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use pdfbull::platform;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// How many days of rotated `pdfbull.log.*` files to keep.
///
/// `tracing_appender` has no size-based rotation — `rolling::never` grows the
/// log without bound, and `rolling::daily` only names one file per day without
/// ever removing them. The files are reaped at startup instead.
const LOG_RETENTION_DAYS: u64 = 7;

/// Delete rotated log files older than [`LOG_RETENTION_DAYS`].
fn prune_old_logs(config_dir: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(config_dir) else {
        return;
    };
    let Some(cutoff) = std::time::SystemTime::now().checked_sub(std::time::Duration::from_secs(
        LOG_RETENTION_DAYS * 24 * 60 * 60,
    )) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        // `rolling::daily` names its files `<prefix>.YYYY-MM-DD`.
        if !name.starts_with("pdfbull.log.") {
            continue;
        }
        if let Ok(meta) = entry.metadata()
            && meta.modified().is_ok_and(|t| t < cutoff)
        {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

fn main() {
    let config_dir = pdfbull::storage::get_config_dir();
    let _ = std::fs::create_dir_all(&config_dir);
    let panic_path = config_dir.join("panic_out.log");

    let panic_path_clone = panic_path.clone();

    let prev_panic_hook = std::panic::take_hook();
    human_panic::setup_panic!();
    let human_panic_hook = std::panic::take_hook();

    // The panic hook writes its own crash report to `panic_out.log`. It used to
    // also append the same message to `pdfbull.log` through a second, raw file
    // handle. That duplicated the `tracing::error!` line the appender below has
    // already written, and the two unsynchronised handles could interleave (or
    // truncate) each other's output.
    std::panic::set_hook(Box::new(move |info| {
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            *s
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.as_str()
        } else {
            "unknown panic"
        };
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());
        let backtrace = std::backtrace::Backtrace::capture();
        let panic_msg = format!(
            "PANIC: {} at {}\nBacktrace:\n{:?}",
            msg, location, backtrace
        );
        tracing::error!(target: "panic", "PANIC at {}: {}\nBacktrace:\n{:?}", location, msg, backtrace);
        let _ = std::fs::write(&panic_path_clone, &panic_msg);
        human_panic_hook(info);
        prev_panic_hook(info);
    }));

    // The tracing appender only offers hourly/daily/never rotation, so the
    // rotated files are reaped here instead of letting them accumulate forever.
    prune_old_logs(&config_dir);

    let file_appender = tracing_appender::rolling::daily(&config_dir, "pdfbull.log");
    let (non_blocking_file, guard) = tracing_appender::non_blocking(file_appender);
    let _guard = Box::leak(Box::new(guard));

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,pdfbull=debug"));
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking_file)
        .with_ansi(false);
    let ring_layer = pdfbull::logging::RingBufferLayer;

    #[cfg(debug_assertions)]
    {
        let stdout_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stdout);
        tracing_subscriber::registry()
            .with(env_filter)
            .with(ring_layer)
            .with(file_layer)
            .with(stdout_layer)
            .init();
    }

    #[cfg(not(debug_assertions))]
    {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(ring_layer)
            .with(file_layer)
            .init();
    }

    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "PDFbull {} - A lightweight, high-performance PDF reader",
            env!("CARGO_PKG_VERSION")
        );
        println!("\nUsage: pdfbull [OPTIONS] [FILE]");
        println!("\nOptions:");
        println!("  -h, --help       Print help information");
        println!("  -V, --version    Print version information");
        return;
    }
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("PDFbull {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    // Feature 10: Deep Windows Integration (Single Instance Mode)
    if let Ok(is_secondary) = platform::ensure_single_instance(&args)
        && is_secondary
    {
        tracing::info!("Sent arguments to main instance. Exiting.");
        return;
    }

    tracing::info!("Starting PDFbull with GPUI-Kit presentation engine...");
    pdfbull::ui_gpui::run_gpui_app();
    // Config writes run on a background thread by design, so anything still
    // queued when the window loop returns was lost on exit — most visibly a
    // `session.json`, `settings.json` or `recent_files.json` change made just
    // before quitting. `flush_config_writes` exists for exactly this and had no
    // caller.
    pdfbull::storage::flush_config_writes();
}
