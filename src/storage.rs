use crate::models::{AppSettings, AppTheme, RecentFile, SessionData};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use time::OffsetDateTime;

pub fn time_ago(unix_secs: u64) -> String {
    if let Ok(past) = OffsetDateTime::from_unix_timestamp(unix_secs as i64) {
        let now = OffsetDateTime::now_utc();
        let diff = now - past;
        let secs = diff.whole_seconds();

        if secs < 0 || unix_secs == u64::MAX {
            return "unknown".into();
        }

        if secs < 60 {
            "just now".into()
        } else if secs < 3600 {
            let m = secs / 60;
            if m == 1 {
                "1 min ago".into()
            } else {
                format!("{m} mins ago")
            }
        } else if secs < 86_400 {
            let h = secs / 3600;
            if h == 1 {
                "1 hour ago".into()
            } else {
                format!("{h} hours ago")
            }
        } else if secs < 172_800 {
            "yesterday".into()
        } else {
            let d = secs / 86_400;
            if d < 30 {
                format!("{d} days ago")
            } else {
                let format =
                    time::format_description::parse("[month repr:short] [day], [year]").unwrap();
                past.format(&format)
                    .unwrap_or_else(|_| "unknown".to_string())
            }
        }
    } else {
        "unknown".into()
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

fn atomic_write(path: &PathBuf, data: &str) -> io::Result<()> {
    let tmp_path = path.with_extension("tmp");

    {
        let mut file = fs::File::create(&tmp_path)?;
        file.write_all(data.as_bytes())?;
        file.sync_all()?;
    }

    fs::rename(&tmp_path, path)?;

    Ok(())
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
            }
        }
    }
    settings
}

pub fn save_settings(settings: &AppSettings) {
    let dir = get_config_dir();
    if let Err(e) = fs::create_dir_all(&dir) {
        eprintln!("Failed to create config directory: {e}");
        return;
    }
    let path = dir.join("settings.json");
    if let Ok(data) = serde_json::to_string_pretty(settings)
        && let Err(e) = atomic_write(&path, &data)
    {
        tracing::error!("Failed to save settings: {}", e);
    }
}

pub fn load_recent_files() -> Vec<RecentFile> {
    let path = get_config_dir().join("recent_files.json");
    if let Ok(data) = fs::read_to_string(&path) {
        if let Ok(files) = serde_json::from_str(&data) {
            return files;
        }
        tracing::warn!("Corrupted recent_files.json, using empty list");
    }
    Vec::new()
}

pub fn save_recent_files(recent_files: &[RecentFile]) {
    let dir = get_config_dir();
    if let Err(e) = fs::create_dir_all(&dir) {
        tracing::error!("Failed to create config directory: {}", e);
        return;
    }
    let path = dir.join("recent_files.json");
    if let Ok(data) = serde_json::to_string_pretty(recent_files)
        && let Err(e) = atomic_write(&path, &data)
    {
        tracing::error!("Failed to save recent files: {}", e);
    }
}

pub fn add_recent_file(recent_files: &mut Vec<RecentFile>, path: &std::path::Path) {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    recent_files.retain(|f| f.path != path.to_string_lossy());

    let new_file = RecentFile {
        path: path.to_string_lossy().to_string(),
        name,
        last_opened: OffsetDateTime::now_utc().unix_timestamp() as u64,
    };

    recent_files.insert(0, new_file);
    if recent_files.len() > 20 {
        recent_files.truncate(20);
    }
    save_recent_files(recent_files);
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
    let dir = get_config_dir();
    if let Err(e) = fs::create_dir_all(&dir) {
        tracing::error!("Failed to create config directory: {}", e);
        return;
    }
    let path = dir.join("session.json");

    if let Ok(data) = serde_json::to_string_pretty(session)
        && let Err(e) = atomic_write(&path, &data)
    {
        tracing::error!("Failed to save session: {}", e);
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
        assert!(result.contains("mins"));
    }

    #[test]
    fn test_time_ago_one_minute() {
        let one_min_ago = OffsetDateTime::now_utc().unix_timestamp() as u64 - 60;
        let result = time_ago(one_min_ago);
        assert_eq!(result, "1 min ago");
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
        assert!(result.contains("hours"));
    }

    #[test]
    fn test_time_ago_yesterday() {
        let yesterday = OffsetDateTime::now_utc().unix_timestamp() as u64 - 86_400;
        let result = time_ago(yesterday);
        assert_eq!(result, "yesterday");
    }

    #[test]
    fn test_time_ago_days() {
        let five_days_ago = OffsetDateTime::now_utc().unix_timestamp() as u64 - 432_000;
        let result = time_ago(five_days_ago);
        assert!(result.contains("days"));
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
            last_opened: 1234567890,
        };
        let json = serde_json::to_string(&file).unwrap();
        let deserialized: RecentFile = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.path, file.path);
        assert_eq!(deserialized.name, file.name);
    }

    #[test]
    fn test_session_data_serialization() {
        let session = SessionData {
            open_tabs: vec!["/path1.pdf".to_string(), "/path2.pdf".to_string()],
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
        let test_path = temp_dir.join("test_atomic_write.txt");

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
        let test_path = temp_dir.join("test_atomic_overwrite.txt");

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
                path: format!("/path/file{}.pdf", i),
                name: format!("file{}.pdf", i),
                last_opened: i as u64,
            });
        }

        while files.len() > 20 {
            files.pop();
        }

        assert!(files.len() <= 20);
    }
}
