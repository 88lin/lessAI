use std::{
    fs,
    path::{Path, PathBuf},
};

use tauri::{AppHandle, Manager};

use crate::models::{AppSettings, DocumentSession};

const SETTINGS_FILE: &str = "settings.json";
const SESSIONS_DIR: &str = "sessions";

fn app_root(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

fn sessions_root(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_root(app)?.join(SESSIONS_DIR);
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

fn session_path(app: &AppHandle, session_id: &str) -> Result<PathBuf, String> {
    Ok(sessions_root(app)?.join(format!("{session_id}.json")))
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&content).map_err(|error| error.to_string())
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let content = serde_json::to_string_pretty(value).map_err(|error| error.to_string())?;
    fs::write(path, content).map_err(|error| error.to_string())
}

pub fn load_settings(app: &AppHandle) -> Result<AppSettings, String> {
    let path = app_root(app)?.join(SETTINGS_FILE);
    if !path.exists() {
        return Ok(AppSettings::default());
    }

    read_json(&path)
}

pub fn save_settings(app: &AppHandle, settings: &AppSettings) -> Result<AppSettings, String> {
    let path = app_root(app)?.join(SETTINGS_FILE);
    write_json(&path, settings)?;
    load_settings(app)
}

pub fn save_session(app: &AppHandle, session: &DocumentSession) -> Result<(), String> {
    let path = session_path(app, &session.id)?;
    write_json(&path, session)
}

pub fn load_session(app: &AppHandle, session_id: &str) -> Result<DocumentSession, String> {
    let path = session_path(app, session_id)?;
    if !path.exists() {
        return Err(format!("未找到会话：{session_id}"));
    }

    read_json(&path)
}

pub fn load_session_optional(
    app: &AppHandle,
    session_id: &str,
) -> Result<Option<DocumentSession>, String> {
    let path = session_path(app, session_id)?;
    if !path.exists() {
        return Ok(None);
    }

    let session = read_json(&path)?;
    Ok(Some(session))
}

pub fn delete_session(app: &AppHandle, session_id: &str) -> Result<(), String> {
    let path = session_path(app, session_id)?;
    if !path.exists() {
        return Ok(());
    }

    fs::remove_file(&path).map_err(|error| error.to_string())
}
