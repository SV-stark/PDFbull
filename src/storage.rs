use crate::models::{AppSettings, AppTheme, RecentFile, SessionData};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use time::OffsetDateTime;

pub fn time_ago(unix_secs: u64) -> String {
    if unix_secs == u64::MAX {
        return "unknown".into();
    }
    let now_ts = OffsetDateTime::now_utc().unix_timestamp();
    if now_ts < 0 {
        return "unknown".into();
    }
    let now = now_ts as u64;
    if unix_secs > now {
        return "unknown".into();
    }
    let diff_secs = now - unix_secs;
    if diff_secs >= 30 * 24 * 3600 {
        if let Ok(past) = OffsetDateTime::from_unix_timestamp(unix_secs as i64) {
            let format = time::macros::format_description!("[month repr:short] [day], [year]");
            past.format(&format)
                .unwrap_or_else(|_| "unknown".to_string())
        } else {
            "unknown".into()
        }
    } else {
        let duration = std::time::Duration::from_secs(diff_secs);
        let res = timeago::Formatter::new().convert(duration);
        if res == "now" {
            "just now".to_string()
        } else {
            res
        }
    }
}

pub fn get_config_dir() -> PathBuf {
    let new_dir = directories::ProjectDirs::from("", "SV-stark", "PDFbull")
        .map(|p| p.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    let old_dir = directories::BaseDirs::new()
        .map(|b| b.config_dir().join("pdfbull"))
        .unwrap_or_else(|| PathBuf::from(".").join("pdfbull"));

    // The migration is a one-time rename that must not be attempted twice
    // concurrently: `get_config_dir` is reachable from the UI thread, the
    // config-writer thread and any background spawn, and two racing renames
    // make the loser report a spurious failure.
    if old_dir.exists() && !new_dir.exists() {
        let _migration = MIGRATION_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // Re-check under the lock: another thread may have migrated while we
        // were waiting for it.
        if old_dir.exists() && !new_dir.exists() {
            if let Err(e) = fs::create_dir_all(new_dir.parent().unwrap_or(&new_dir)) {
                tracing::warn!("Failed to create parent dir for migration: {}", e);
            }
            if let Err(e) = fs::rename(&old_dir, &new_dir) {
                tracing::warn!(
                    "Failed to migrate old config from {:?} to {:?}: {}",
                    old_dir,
                    new_dir,
                    e
                );
            } else {
                tracing::info!("Migrated config from {:?} to {:?}", old_dir, new_dir);
            }
        }
    }

    new_dir
}

fn atomic_write(path: &Path, data: &str) -> io::Result<()> {
    atomic_write_bytes(path, data.as_bytes())
}

/// How many times to retry the rename before giving up, and how long to wait
/// between attempts.
///
/// The rename fails with a sharing violation whenever the destination is open
/// in another process. That is frequently transient: antivirus scanners and
/// Search indexers hold a PDF for a few hundred milliseconds at a time, so a
/// short bounded retry turns most of those into an invisible success.
const RENAME_ATTEMPTS: u32 = 10;
const RENAME_BACKOFF_MS: u64 = 40;

/// Write `data` to `path` via a temp file in the same directory followed by a
/// rename, so a failure part-way through never leaves a truncated file behind.
/// Callers that overwrite a user's document (e.g. saving annotations into an
/// existing PDF) must go through this rather than `fs::write`.
///
/// There is deliberately **no** non-atomic fallback. The previous version fell
/// back to `fs::write`, which truncates the destination before streaming into
/// it — and the common reason for `persist` to fail is precisely that the
/// target is locked by another process, so the fallback would truncate the
/// user's only copy and then usually fail mid-stream anyway, leaving a
/// zero-length PDF. Failing loudly is recoverable; silently destroying the
/// original is not.
pub fn atomic_write_bytes(path: &Path, data: &[u8]) -> io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!(
                "could not create a temporary file next to {}: {e}",
                path.display()
            ),
        )
    })?;

    temp.write_all(data)?;

    // Flush the handle and force it to disk *before* the rename. Without this,
    // a crash between the rename and the page-cache flush leaves a
    // correctly-named, zero-length file where the document used to be.
    temp.flush()?;
    temp.as_file().sync_all()?;

    for attempt in 1..=RENAME_ATTEMPTS {
        // `persist` consumes the handle, but hands it back inside the error, so
        // a failed attempt can be retried without re-reading `data`.
        match temp.persist(path) {
            Ok(_) => return Ok(()),
            Err(e) => {
                if attempt == RENAME_ATTEMPTS {
                    return Err(io::Error::new(
                        e.error.kind(),
                        format!(
                            "could not replace {}: {}. The file is most likely open in \
                             another application — close it and try again. The existing \
                             file was left untouched.",
                            path.display(),
                            e.error
                        ),
                    ));
                }
                temp = e.file;
                std::thread::sleep(std::time::Duration::from_millis(RENAME_BACKOFF_MS));
            }
        }
    }

    unreachable!("the loop returns on its final iteration")
}

pub fn load_settings() -> AppSettings {
    let mut settings = AppSettings::default();
    let path = get_config_dir().join("settings.json");
    if let Ok(data) = fs::read_to_string(&path) {
        if let Ok(loaded) = serde_json::from_str::<AppSettings>(&data) {
            settings = loaded;
        } else {
            tracing::warn!("Corrupted settings.json, using defaults");
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&data)
                && let Some(obj) = value.as_object()
            {
                if let Some(theme) = obj.get("theme").and_then(|v| v.as_str()) {
                    settings.theme = match theme {
                        "Light" => AppTheme::Light,
                        "Dark" => AppTheme::Dark,
                        _ => AppTheme::System,
                    };
                }
                if let Some(v) = obj.get("auto_save").and_then(serde_json::Value::as_bool) {
                    settings.auto_save = v;
                }
                if let Some(v) = obj.get("default_zoom").and_then(serde_json::Value::as_f64) {
                    settings.default_zoom = v as f32;
                }
                if let Some(v) = obj
                    .get("show_logs_on_start")
                    .and_then(serde_json::Value::as_bool)
                {
                    settings.show_logs_on_start = v;
                }
            }
        }
    }
    settings
}

/// Serializes the one-time config-directory migration, which `get_config_dir`
/// may be entered from any thread (the UI reads settings while a background
/// write is in flight).
static MIGRATION_LOCK: Mutex<()> = Mutex::new(());

/// A queued config write.
enum ConfigWrite {
    Write {
        name: String,
        json: String,
    },
    /// Resolve only once every job queued *before* this one has hit the disk.
    /// The zero-capacity channel is what makes it a real barrier: the writer
    /// blocks on `recv` until we block on `recv`.
    Flush(std::sync::mpsc::SyncSender<()>),
}

/// All config writes funnel through **one** dedicated thread.
///
/// The previous implementation spawned an OS thread per save and serialized
/// only the write itself via a mutex. That serializes but does not *order*:
/// two saves racing for the mutex can land out of order, so a writer holding an
/// older snapshot can clobber a newer one. Opening two files quickly produced a
/// `recent_files.json` missing the file that was opened most recently — invisible
/// because the in-memory cache stayed correct, so the entry only vanished on
/// restart. A single FIFO consumer makes the order the call order by
/// construction.
static CONFIG_WRITER: LazyLock<std::sync::mpsc::Sender<ConfigWrite>> = LazyLock::new(|| {
    let (tx, rx) = std::sync::mpsc::channel::<ConfigWrite>();
    std::thread::Builder::new()
        .name("pdfbull-config-writer".into())
        .spawn(move || {
            while let Ok(job) = rx.recv() {
                match job {
                    ConfigWrite::Write { name, json } => write_json_now(&name, &json),
                    ConfigWrite::Flush(ack) => {
                        let _ = ack.send(());
                    }
                }
            }
        })
        .expect("failed to spawn the config-writer thread");
    tx
});

/// Do one config write. Only ever called from the config-writer thread, so the
/// temp-file-then-rename sequence for a given path needs no extra locking.
fn write_json_now(name: &str, json: &str) {
    let dir = get_config_dir();
    if let Err(e) = fs::create_dir_all(&dir) {
        tracing::error!("Failed to create config directory: {}", e);
        return;
    }
    if let Err(e) = atomic_write(&dir.join(name), json) {
        tracing::error!("Failed to save {name}: {e}");
    }
}

/// Write `json` to `name` inside the config dir on a background thread, with
/// writes serialized *and* ordered.
fn write_json_in_background(name: &str, json: String) {
    let job = ConfigWrite::Write {
        name: name.to_string(),
        json,
    };
    if CONFIG_WRITER.send(job).is_err() {
        tracing::error!("Config writer thread is gone; {name} was not saved");
    }
}

/// Block until every config write queued so far has reached the disk.
///
/// Writes are asynchronous by design so the UI never stalls on disk I/O, which
/// meant a process exiting with a save still in flight silently lost
/// `session.json`, `settings.json` and `recent_files.json`. Call this on the
/// way out of `main`.
pub fn flush_config_writes() {
    let (tx, rx) = std::sync::mpsc::sync_channel(0);
    if CONFIG_WRITER.send(ConfigWrite::Flush(tx)).is_err() {
        return;
    }
    let _ = rx.recv();
}

pub fn save_settings(settings: &AppSettings) {
    let settings = settings.clone();
    if let Ok(data) = serde_json::to_string_pretty(&settings) {
        write_json_in_background("settings.json", data);
    }
}

/// Single source of truth for the recent-file list, cached after the first
/// disk read. Keeping it here (rather than read-modify-write on a caller-owned
/// `Vec`) means opening N files at once can't race and silently drop entries.
static RECENT_FILES: LazyLock<Mutex<Option<Vec<RecentFile>>>> = LazyLock::new(|| Mutex::new(None));

fn recent_files_lock() -> std::sync::MutexGuard<'static, Option<Vec<RecentFile>>> {
    RECENT_FILES.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn load_recent_files() -> Vec<RecentFile> {
    let mut guard = recent_files_lock();
    if let Some(list) = guard.as_ref() {
        return list.clone();
    }
    let path = get_config_dir().join("recent_files.json");
    let loaded = match fs::read_to_string(&path) {
        Ok(data) => match serde_json::from_str::<Vec<RecentFile>>(&data) {
            Ok(files) => files,
            Err(_) => {
                tracing::warn!("Corrupted recent_files.json, using empty list");
                Vec::new()
            }
        },
        Err(_) => Vec::new(),
    };
    *guard = Some(loaded.clone());
    loaded
}

/// Record `path` as the most recently opened file, de-duplicating any earlier
/// entry for the same path and capping the list at 20 items.
pub fn add_recent_file(path: &Path) {
    let snapshot = {
        let mut guard = recent_files_lock();
        let list = guard.get_or_insert_with(Vec::new);
        let path_str = path.to_string_lossy().to_string();

        list.retain(|f| f.path != path_str);
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        list.insert(
            0,
            RecentFile {
                path: path_str,
                name,
                last_opened: OffsetDateTime::now_utc().unix_timestamp().max(0) as u64,
            },
        );
        if list.len() > 20 {
            list.truncate(20);
        }
        list.clone()
    };

    if let Ok(json) = serde_json::to_string_pretty(&snapshot) {
        write_json_in_background("recent_files.json", json);
    }
}

pub fn save_recent_files(recent_files: &[RecentFile]) {
    *recent_files_lock() = Some(recent_files.to_vec());
    if let Ok(json) = serde_json::to_string_pretty(recent_files) {
        write_json_in_background("recent_files.json", json);
    }
}

pub fn load_session() -> Option<SessionData> {
    let path = get_config_dir().join("session.json");
    if let Ok(data) = fs::read_to_string(&path) {
        match serde_json::from_str::<SessionData>(&data) {
            Ok(session) => return Some(session),
            Err(e) => {
                tracing::warn!("Corrupted session.json: {}", e);
                let _ = fs::rename(&path, path.with_extension("bak"));
            }
        }
    }
    None
}

pub fn save_session(session: &SessionData) {
    if let Ok(data) = serde_json::to_string_pretty(session) {
        write_json_in_background("session.json", data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_ago_just_now() {
        let now = OffsetDateTime::now_utc().unix_timestamp() as u64;
        let result = time_ago(now);
        assert_eq!(result, "just now");
    }

    #[test]
    fn test_time_ago_minutes() {
        let two_mins_ago = OffsetDateTime::now_utc().unix_timestamp() as u64 - 120;
        let result = time_ago(two_mins_ago);
        assert!(result.contains("minute"));
    }

    #[test]
    fn test_time_ago_one_minute() {
        let one_min_ago = OffsetDateTime::now_utc().unix_timestamp() as u64 - 60;
        let result = time_ago(one_min_ago);
        assert_eq!(result, "1 minute ago");
    }

    #[test]
    fn test_time_ago_one_hour() {
        let one_hour_ago = OffsetDateTime::now_utc().unix_timestamp() as u64 - 3600;
        let result = time_ago(one_hour_ago);
        assert_eq!(result, "1 hour ago");
    }

    #[test]
    fn test_time_ago_hours() {
        let three_hours_ago = OffsetDateTime::now_utc().unix_timestamp() as u64 - 10_800;
        let result = time_ago(three_hours_ago);
        assert!(result.contains("hour"));
    }

    #[test]
    fn test_time_ago_yesterday() {
        let yesterday = OffsetDateTime::now_utc().unix_timestamp() as u64 - 86_400;
        let result = time_ago(yesterday);
        assert_eq!(result, "1 day ago");
    }

    #[test]
    fn test_time_ago_days() {
        let five_days_ago = OffsetDateTime::now_utc().unix_timestamp() as u64 - 432_000;
        let result = time_ago(five_days_ago);
        assert!(result.contains("day"));
    }

    #[test]
    fn test_time_ago_unknown_timestamp() {
        let result = time_ago(u64::MAX);
        assert_eq!(result, "unknown");
    }

    #[test]
    fn test_time_ago_future_timestamp() {
        let future = OffsetDateTime::now_utc().unix_timestamp() as u64 + 10000;
        let result = time_ago(future);
        assert_eq!(result, "unknown");
    }

    #[test]
    fn test_app_settings_serialization() {
        let settings = AppSettings::default();
        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: AppSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.theme, settings.theme);
        assert_eq!(deserialized.cache_size, settings.cache_size);
    }

    #[test]
    fn test_recent_file_serialization() {
        let file = RecentFile {
            path: "/test/file.pdf".to_string(),
            name: "file.pdf".to_string(),
            last_opened: 1_234_567_890,
        };
        let json = serde_json::to_string(&file).unwrap();
        let deserialized: RecentFile = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.path, file.path);
        assert_eq!(deserialized.name, file.name);
    }

    #[test]
    fn test_session_data_serialization() {
        let session = SessionData {
            open_tabs: vec![
                crate::models::SessionTabEntry::Simple("/path1.pdf".to_string()),
                crate::models::SessionTabEntry::Simple("/path2.pdf".to_string()),
            ],
            active_tab: 1,
        };
        let json = serde_json::to_string(&session).unwrap();
        let deserialized: SessionData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.open_tabs.len(), 2);
        assert_eq!(deserialized.active_tab, 1);
    }

    #[test]
    fn test_session_data_empty_tabs() {
        let session = SessionData::default();
        let json = serde_json::to_string(&session).unwrap();
        let deserialized: SessionData = serde_json::from_str(&json).unwrap();
        assert!(deserialized.open_tabs.is_empty());
        assert_eq!(deserialized.active_tab, 0);
    }

    #[test]
    fn test_atomic_write_creates_temp_file() {
        use std::io::Read;

        let temp_dir = std::env::temp_dir();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let test_path = temp_dir.join(format!("test_atomic_write_{}.txt", timestamp));

        let result = atomic_write(&test_path, "test content");
        assert!(result.is_ok());
        assert!(test_path.exists());

        let mut file = std::fs::File::open(&test_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        assert_eq!(contents, "test content");

        let _ = std::fs::remove_file(&test_path);
    }

    #[test]
    fn test_atomic_write_overwrites() {
        use std::io::Read;

        let temp_dir = std::env::temp_dir();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let test_path = temp_dir.join(format!("test_atomic_overwrite_{}.txt", timestamp));

        let _ = atomic_write(&test_path, "original");
        let result = atomic_write(&test_path, "updated");
        assert!(result.is_ok());

        let mut file = std::fs::File::open(&test_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        assert_eq!(contents, "updated");

        let _ = std::fs::remove_file(&test_path);
    }

    #[test]
    fn test_get_config_dir_returns_path() {
        let dir = get_config_dir();
        assert!(dir.to_string_lossy().contains("PDFbull") || dir.to_string_lossy() == ".");
    }

    #[test]
    fn test_recent_files_truncation() {
        let mut files = Vec::new();
        for i in 0..25 {
            files.push(RecentFile {
                path: format!("/path/file{i}.pdf"),
                name: format!("file{i}.pdf"),
                last_opened: i as u64,
            });
        }

        while files.len() > 20 {
            files.pop();
        }

        assert!(files.len() <= 20);
    }
}
