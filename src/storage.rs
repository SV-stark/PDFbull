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

        if secs < 60 {
            "just now".into()
        } else if secs < 3600 {
            let m = secs / 60;
            if m == 1 {
                "1 min ago".into()
            } else {
                format!("{} mins ago", m)
            }
        } else if secs < 86400 {
            let h = secs / 3600;
            if h == 1 {
                "1 hour ago".into()
            } else {
                format!("{} hours ago", h)
            }
        } else if secs < 172800 {
            "yesterday".into()
        } else {
            let d = secs / 86400;
            if d < 30 {
                format!("{} days ago", d)
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
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&data) {
                if let Some(obj) = value.as_object() {
                    if let Some(theme) = obj.get("theme").and_then(|v| v.as_str()) {
                        settings.theme = match theme {
                            "Light" => AppTheme::Light,
                            "Dark" => AppTheme::Dark,
                            _ => AppTheme::System,
                        };
                    }
                    if let Some(v) = obj.get("auto_save").and_then(|v| v.as_bool()) {
                        settings.auto_save = v;
                    }
                    if let Some(v) = obj.get("default_zoom").and_then(|v| v.as_f64()) {
                        settings.default_zoom = v as f32;
                    }
                }
            }
        }
    }
    settings
}

pub fn save_settings(settings: &AppSettings) {
    let dir = get_config_dir();
    if let Err(e) = fs::create_dir_all(&dir) {
        eprintln!("Failed to create config directory: {}", e);
        return;
    }
    let path = dir.join("settings.json");
    if let Ok(data) = serde_json::to_string_pretty(settings) {
        if let Err(e) = atomic_write(&path, &data) {
            tracing::error!("Failed to save settings: {}", e);
        }
    }
}

pub fn load_recent_files() -> Vec<RecentFile> {
    let path = get_config_dir().join("recent_files.json");
    if let Ok(data) = fs::read_to_string(&path) {
        if let Ok(files) = serde_json::from_str(&data) {
            return files;
        } else {
            tracing::warn!("Corrupted recent_files.json, using empty list");
        }
    }
    Vec::new()
}

pub fn save_recent_files(recent_files: &Vec<RecentFile>) {
    let dir = get_config_dir();
    if let Err(e) = fs::create_dir_all(&dir) {
        tracing::error!("Failed to create config directory: {}", e);
        return;
    }
    let path = dir.join("recent_files.json");
    if let Ok(data) = serde_json::to_string_pretty(recent_files) {
        if let Err(e) = atomic_write(&path, &data) {
            tracing::error!("Failed to save recent files: {}", e);
        }
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

    if let Ok(data) = serde_json::to_string_pretty(session) {
        if let Err(e) = atomic_write(&path, &data) {
            tracing::error!("Failed to save session: {}", e);
        }
    }
}
