use crate::models::{AppSettings, AppTheme, RecentFile, SessionData};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

pub fn get_config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("pdfbull")
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
            eprintln!("Warning: Corrupted settings.json, using defaults");
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&data) {
                if let Some(obj) = value.as_object() {
                    if let Some(theme) = obj.get("theme").and_then(|v| v.as_str()) {
                        settings.theme = AppTheme::from(theme);
                    }
                    if let Some(v) = obj.get("auto_save").and_then(|v| v.as_bool()) {
                        settings.auto_save = v;
                    }
                    if let Some(v) = obj.get("remember_last_file").and_then(|v| v.as_bool()) {
                        settings.remember_last_file = v;
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
            eprintln!("Failed to save settings: {}", e);
        }
    }
}

pub fn load_recent_files() -> Vec<RecentFile> {
    let path = get_config_dir().join("recent_files.json");
    if let Ok(data) = fs::read_to_string(&path) {
        if let Ok(files) = serde_json::from_str(&data) {
            return files;
        } else {
            eprintln!("Warning: Corrupted recent_files.json, using empty list");
        }
    }
    Vec::new()
}

pub fn save_recent_files(recent_files: &Vec<RecentFile>) {
    let dir = get_config_dir();
    if let Err(e) = fs::create_dir_all(&dir) {
        eprintln!("Failed to create config directory: {}", e);
        return;
    }
    let path = dir.join("recent_files.json");
    if let Ok(data) = serde_json::to_string_pretty(recent_files) {
        if let Err(e) = atomic_write(&path, &data) {
            eprintln!("Failed to save recent files: {}", e);
        }
    }
}

pub fn add_recent_file(recent_files: &mut Vec<RecentFile>, path: &PathBuf) {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    recent_files.retain(|f| f.path != path.to_string_lossy());

    let new_file = RecentFile {
        path: path.to_string_lossy().to_string(),
        name,
        last_opened: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    };

    recent_files.insert(0, new_file);
    if recent_files.len() > 10 {
        recent_files.truncate(10);
    }
    save_recent_files(recent_files);
}

pub fn load_session() -> Option<SessionData> {
    let path = get_config_dir().join("session.json");
    if let Ok(data) = fs::read_to_string(&path) {
        match serde_json::from_str::<SessionData>(&data) {
            Ok(session) => return Some(session),
            Err(e) => eprintln!("Warning: Corrupted session.json: {}", e),
        }
    }
    None
}

pub fn save_session(session: &SessionData) {
    let dir = get_config_dir();
    if let Err(e) = fs::create_dir_all(&dir) {
        eprintln!("Failed to create config directory: {}", e);
        return;
    }
    let path = dir.join("session.json");

    if let Ok(data) = serde_json::to_string_pretty(session) {
        if let Err(e) = atomic_write(&path, &data) {
            eprintln!("Failed to save session: {}", e);
        }
    }
}
