use std::{
    fs,
    path::{Path, PathBuf},
};

use tauri::{AppHandle, Manager};

use crate::{
    atomic_write::write_bytes_atomically,
    models::{AppSettings, DocumentSession},
    settings_validation::validate_numeric_settings,
};

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
    let content = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    write_json_bytes(path, &content)
}

fn write_json_bytes(path: &Path, payload: &[u8]) -> Result<(), String> {
    write_bytes_atomically(path, payload)?;
    restrict_json_file_permissions(path);
    Ok(())
}

fn restrict_json_file_permissions(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // settings/session 可能包含敏感信息（例如 API Key、草稿内容），尽量限制文件权限。
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

pub fn load_settings(app: &AppHandle) -> Result<AppSettings, String> {
    let path = app_root(app)?.join(SETTINGS_FILE);
    if !path.exists() {
        return Ok(AppSettings::default());
    }

    let settings = read_json(&path)?;
    validate_numeric_settings(&settings)?;
    Ok(settings)
}

pub fn save_settings(app: &AppHandle, settings: &AppSettings) -> Result<AppSettings, String> {
    let path = app_root(app)?.join(SETTINGS_FILE);
    validate_numeric_settings(settings)?;
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

#[cfg(test)]
mod tests {
    use std::fs;

    use super::validate_numeric_settings;
    use crate::{
        models::AppSettings,
        test_support::{cleanup_dir, unique_test_dir},
    };

    #[test]
    fn write_json_bytes_creates_parent_dirs_and_writes_payload() {
        let root = unique_test_dir("json-bytes-create");
        let target = root.join("nested").join("data.json");

        super::write_json_bytes(&target, br#"{"key":"value"}"#)
            .expect("expected json bytes helper to write payload");

        let stored = fs::read(&target).expect("read written json");
        assert_eq!(stored, br#"{"key":"value"}"#);
        cleanup_dir(&root);
    }

    #[test]
    fn write_json_bytes_replaces_existing_payload() {
        let root = unique_test_dir("json-bytes-replace");
        fs::create_dir_all(&root).expect("create root");
        let target = root.join("data.json");
        fs::write(&target, br#"{"old":true}"#).expect("seed old json");

        super::write_json_bytes(&target, br#"{"new":true}"#)
            .expect("expected json bytes helper to replace payload");

        let stored = fs::read(&target).expect("read replaced json");
        assert_eq!(stored, br#"{"new":true}"#);
        cleanup_dir(&root);
    }

    #[test]
    fn validate_numeric_settings_rejects_zero_chunks_per_request() {
        let mut settings = AppSettings::default();
        settings.chunks_per_request = 0;

        let error = validate_numeric_settings(&settings).expect_err("expected invalid batch size");

        assert_eq!(error, "单次请求处理块数必须大于等于 1。");
    }
}
