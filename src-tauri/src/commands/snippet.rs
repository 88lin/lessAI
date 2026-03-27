use std::path::Path;

use tauri::{AppHandle, State};

use crate::{
    documents::document_format,
    rewrite,
    state::{with_session_lock, AppState},
    storage,
};

#[tauri::command]
pub async fn rewrite_snippet(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    text: String,
) -> Result<String, String> {
    if text.trim().is_empty() {
        return Err("选区内容为空。".to_string());
    }

    {
        // 避免与后台 job 竞争使用同一 session（尤其是自动批处理还在跑的时候）。
        let jobs = state
            .jobs
            .lock()
            .map_err(|_| "任务状态锁已损坏。".to_string())?;
        if jobs.contains_key(&session_id) {
            return Err("后台任务仍在运行或正在退出，请稍后再试。".to_string());
        }
    }

    let session = with_session_lock(state.inner(), &session_id, || {
        storage::load_session(&app, &session_id)
    })?;

    let settings = storage::load_settings(&app)?;
    let format = document_format(Path::new(&session.document_path));
    rewrite::rewrite_chunk(&settings, &text, format).await
}

