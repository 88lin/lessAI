use tauri::AppHandle;

use crate::{
    models::DocumentSession,
    rewrite,
    rewrite_batch_commit::{batch_commit_mode, commit_rewrite_result, emit_chunk_completed_events},
    rewrite_writeback::validate_candidate_batch_writeback,
    state::AppState,
    storage,
};

use super::{load_rewriteable_session, prepare_loaded_rewrite_batch};

pub(crate) async fn process_chunk(
    app: &AppHandle,
    state: &AppState,
    session_id: &str,
    index: usize,
    auto_approve: bool,
) -> Result<(), String> {
    process_chunk_batch(app, state, session_id, &[index], auto_approve).await
}

async fn process_chunk_batch(
    app: &AppHandle,
    state: &AppState,
    session_id: &str,
    indices: &[usize],
    auto_approve: bool,
) -> Result<(), String> {
    if indices.is_empty() {
        return Ok(());
    }

    let session = load_rewriteable_session(app, state, session_id)?;
    process_loaded_chunk_batch(app, state, session_id, &session, indices, auto_approve).await
}

pub(super) async fn process_loaded_chunk_batch(
    app: &AppHandle,
    state: &AppState,
    session_id: &str,
    session: &DocumentSession,
    indices: &[usize],
    auto_approve: bool,
) -> Result<(), String> {
    let settings = storage::load_settings(app)?;
    let prepared = prepare_loaded_rewrite_batch(session, indices)?;
    crate::rewrite_job_state::mark_chunks_running(app, state, session_id, indices)?;

    let completed_batch = commit_rewrite_result(
        app,
        state,
        session_id,
        indices,
        rewrite::rewrite_chunks(&settings, &prepared.source_texts, prepared.format).await,
        batch_commit_mode(auto_approve),
        validate_candidate_batch_writeback,
    )?;
    emit_chunk_completed_events(app, session_id, &completed_batch)?;
    Ok(())
}
