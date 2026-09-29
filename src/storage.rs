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

    new_dir
}

fn atomic_write(path: &Path, data: &str) -> io::Result<()> {
    atomic_write_bytes(path, data.as_bytes())
}

/// Write `data` to `path` via a temp file in the same directory followed by a
/// rename, so a failure part-way through never leaves a truncated file behind.
/// Callers that overwrite a user's document (e.g. saving annotations into an
/// existing PDF) must go through this rather than `fs::write`.
pub fn atomic_write_bytes(path: &Path, data: &[u8]) -> io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    if let Ok(mut temp) = tempfile::NamedTempFile::new_in(parent)
        && temp.write_all(data).is_ok()
        && temp.persist(path).is_ok()
    {
        return Ok(());
    }
    // Fallback in case of cross-device links, junctions, or persist failures
    fs::write(path, data)
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

/// Serializes the background config writers, and the one-time config-directory
/// migration. Without this, two concurrent saves can interleave their
/// temp-file-then-rename on the same path, and two threads can both attempt the
/// legacy-directory migration.
static WRITE_SERIAL: Mutex<()> = Mutex::new(());

/// Write `json` to `name` inside the config dir on a background thread, with
/// writes serialized.
fn write_json_in_background(name: &str, json: String) {
    let name = name.to_string();
    std::thread::spawn(move || {
        // Held across `get_config_dir` so the one-time legacy-directory
        // migration cannot be run twice concurrently.
        let _serial = WRITE_SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let dir = get_config_dir();
        if let Err(e) = fs::create_dir_all(&dir) {
            tracing::error!("Failed to create config directory: {}", e);
            return;
        }
        if let Err(e) = atomic_write(&dir.join(&name), &json) {
            tracing::error!("Failed to save {name}: {e}");
        }
    });
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
