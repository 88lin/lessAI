use std::path::Path;

use tauri::{AppHandle, State};

use crate::{
    document_snapshot::{
        ensure_document_snapshot_matches, SNAPSHOT_MISMATCH_ERROR, SNAPSHOT_MISSING_ERROR,
    },
    documents::WritebackMode,
    editor_writeback::{execute_editor_writeback, EditorWritebackPayload},
    models::{DocumentSession, DocumentSnapshot},
    persist,
    session_access::{access_current_session, CurrentSessionRequest},
    state::AppState,
    storage,
};

pub(crate) const EDITOR_BASE_SNAPSHOT_MISSING_ERROR: &str =
    "当前编辑器缺少打开时的文件快照，无法确认保存安全性。请重新进入编辑模式后再试。";
pub(crate) const EDITOR_BASE_SNAPSHOT_EXPIRED_ERROR: &str =
    "编辑器基准已过期，原文件已在外部发生变化。请重新进入编辑模式后再试。";
pub(crate) const ACTIVE_EDITOR_SESSION_ERROR: &str =
    "当前文档正在执行自动任务，请先暂停并取消后再继续编辑。";

pub(crate) fn ensure_editor_base_snapshot_matches_path(
    path: &Path,
    editor_base_snapshot: Option<&DocumentSnapshot>,
) -> Result<(), String> {
    match ensure_document_snapshot_matches(path, editor_base_snapshot) {
        Ok(_) => Ok(()),
        Err(error) if error == SNAPSHOT_MISSING_ERROR => {
            Err(EDITOR_BASE_SNAPSHOT_MISSING_ERROR.to_string())
        }
        Err(error) if error == SNAPSHOT_MISMATCH_ERROR => {
            Err(EDITOR_BASE_SNAPSHOT_EXPIRED_ERROR.to_string())
        }
        Err(error) => Err(error),
    }
}

pub(crate) fn with_idle_editor_session<T, Run>(
    app: &AppHandle,
    state: &State<'_, AppState>,
    session_id: &str,
    editor_base_snapshot: &Option<DocumentSnapshot>,
    run: Run,
) -> Result<T, String>
where
    Run: FnOnce(DocumentSession) -> Result<T, String>,
{
    access_current_session(
        CurrentSessionRequest::guarded_refresh(
            app,
            state.inner(),
            session_id,
            |session: &DocumentSession| {
                ensure_editor_base_snapshot_matches_path(
                    Path::new(&session.document_path),
                    editor_base_snapshot.as_ref(),
                )
            },
        )
        .with_active_job_error(ACTIVE_EDITOR_SESSION_ERROR),
        run,
    )
}

pub(crate) fn rebuild_clean_session_from_disk(
    app: &AppHandle,
    existing: &DocumentSession,
) -> Result<DocumentSession, String> {
    crate::session_loader::load_clean_session_from_existing(
        app,
        existing,
        existing.created_at,
        false,
    )
}

pub(crate) fn persist_rebuilt_editor_session(
    app: &AppHandle,
    session: DocumentSession,
) -> Result<DocumentSession, String> {
    let rebuilt = rebuild_clean_session_from_disk(app, &session)?;
    persist::save_and_return(rebuilt, |rebuilt| storage::save_session(app, rebuilt))
}

pub(crate) fn run_loaded_editor_writeback<T, Build, Execute, Finish>(
    session: DocumentSession,
    mode: WritebackMode,
    build: Build,
    execute: Execute,
    finish: Finish,
) -> Result<T, String>
where
    Build: FnOnce(&DocumentSession) -> Result<EditorWritebackPayload, String>,
    Execute: FnOnce(&DocumentSession, &EditorWritebackPayload, WritebackMode) -> Result<(), String>,
    Finish: FnOnce(DocumentSession) -> Result<T, String>,
{
    let payload = build(&session)?;
    execute(&session, &payload, mode)?;
    finish(session)
}

pub(crate) fn run_idle_editor_writeback<T, Build, Finish>(
    app: &AppHandle,
    state: &State<'_, AppState>,
    session_id: &str,
    editor_base_snapshot: &Option<DocumentSnapshot>,
    mode: WritebackMode,
    build: Build,
    finish: Finish,
) -> Result<T, String>
where
    Build: FnOnce(&DocumentSession) -> Result<EditorWritebackPayload, String>,
    Finish: FnOnce(DocumentSession) -> Result<T, String>,
{
    with_idle_editor_session(app, state, session_id, editor_base_snapshot, |session| {
        run_loaded_editor_writeback(session, mode, build, execute_editor_writeback, finish)
    })
}

#[cfg(test)]
#[path = "editor_session_tests.rs"]
mod tests;
