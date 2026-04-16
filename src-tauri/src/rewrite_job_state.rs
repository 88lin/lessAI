use tauri::AppHandle;

use crate::{
    models::{ChunkStatus, DocumentSession, RunningState},
    rewrite_permissions::CHUNK_INDEX_OUT_OF_RANGE_ERROR,
    session_access::CurrentSessionRequest,
    session_edit::{mutate_session_cloned_now, mutate_session_now, save_session_value},
    state::AppState,
};

pub(crate) fn clear_running_chunks(session: &mut DocumentSession) -> bool {
    update_running_chunks(session, ChunkStatus::Idle, None)
}

pub(crate) fn fail_running_chunks(session: &mut DocumentSession, error: &str) -> bool {
    update_running_chunks(session, ChunkStatus::Failed, Some(error))
}

fn update_running_chunks(
    session: &mut DocumentSession,
    status: ChunkStatus,
    error_message: Option<&str>,
) -> bool {
    let mut touched = false;
    let error_message = error_message.map(str::to_string);
    for chunk in &mut session.chunks {
        if chunk.status != ChunkStatus::Running {
            continue;
        }
        chunk.status = status;
        chunk.error_message = error_message.clone();
        touched = true;
    }
    touched
}

pub(crate) fn update_target_chunks(
    session: &mut DocumentSession,
    indices: &[usize],
    status: ChunkStatus,
    error_message: Option<&str>,
) -> Result<(), String> {
    for index in indices.iter().copied() {
        if session.chunks.get(index).is_none() {
            return Err(CHUNK_INDEX_OUT_OF_RANGE_ERROR.to_string());
        }
    }

    let error_message = error_message.map(str::to_string);
    for index in indices.iter().copied() {
        let chunk = session
            .chunks
            .get_mut(index)
            .ok_or_else(|| CHUNK_INDEX_OUT_OF_RANGE_ERROR.to_string())?;
        chunk.status = status;
        chunk.error_message = error_message.clone();
    }
    Ok(())
}

pub(crate) fn set_chunks_running_status(
    session: &mut DocumentSession,
    indices: &[usize],
) -> Result<(), String> {
    update_target_chunks(session, indices, ChunkStatus::Running, None)?;
    if session.status != RunningState::Paused {
        session.status = RunningState::Running;
    }
    Ok(())
}

pub(crate) fn fail_target_chunks_and_reset_other_running(
    session: &mut DocumentSession,
    indices: &[usize],
    error: &str,
) -> Result<(), String> {
    update_target_chunks(session, indices, ChunkStatus::Failed, Some(error))?;
    session.status = RunningState::Failed;
    clear_running_chunks(session);
    Ok(())
}

pub(crate) fn compute_session_state(session: &DocumentSession) -> RunningState {
    if session
        .chunks
        .iter()
        .any(|chunk| chunk.status == ChunkStatus::Failed)
    {
        return RunningState::Failed;
    }

    if session
        .chunks
        .iter()
        .all(|chunk| chunk.status == ChunkStatus::Done)
    {
        return RunningState::Completed;
    }

    RunningState::Idle
}

pub(crate) fn set_session_cancelled(session: &mut DocumentSession) {
    session.status = RunningState::Cancelled;
    clear_running_chunks(session);
}

pub(crate) fn set_session_paused(session: &mut DocumentSession) {
    session.status = RunningState::Paused;
}

pub(crate) fn set_session_running(session: &mut DocumentSession) {
    session.status = RunningState::Running;
}

fn mutate_rewrite_job_session_now<T, Mutate>(
    app: &AppHandle,
    state: &AppState,
    session_id: &str,
    mutate: Mutate,
) -> Result<T, String>
where
    Mutate: FnOnce(
        &mut DocumentSession,
        chrono::DateTime<chrono::Utc>,
    ) -> Result<crate::session_edit::SessionMutation<T>, String>,
{
    mutate_session_now(
        CurrentSessionRequest::stored(app, state, session_id),
        mutate,
    )
}

fn mutate_rewrite_job_session_cloned_now<Mutate>(
    app: &AppHandle,
    state: &AppState,
    session_id: &str,
    mutate: Mutate,
) -> Result<DocumentSession, String>
where
    Mutate: FnOnce(&mut DocumentSession, chrono::DateTime<chrono::Utc>) -> Result<(), String>,
{
    mutate_session_cloned_now(
        CurrentSessionRequest::stored(app, state, session_id),
        mutate,
    )
}

pub(crate) fn mark_session_cancelled(
    app: &AppHandle,
    state: &AppState,
    session_id: &str,
) -> Result<DocumentSession, String> {
    mutate_rewrite_job_session_cloned_now(app, state, session_id, |session, _| {
        set_session_cancelled(session);
        Ok(())
    })
}

pub(crate) fn mark_session_paused(
    app: &AppHandle,
    state: &AppState,
    session_id: &str,
) -> Result<DocumentSession, String> {
    mutate_rewrite_job_session_cloned_now(app, state, session_id, |session, _| {
        set_session_paused(session);
        Ok(())
    })
}

pub(crate) fn mark_session_running(
    app: &AppHandle,
    state: &AppState,
    session_id: &str,
) -> Result<DocumentSession, String> {
    mutate_rewrite_job_session_cloned_now(app, state, session_id, |session, _| {
        set_session_running(session);
        Ok(())
    })
}

pub(crate) fn finalize_auto_session(
    app: &AppHandle,
    state: &AppState,
    session_id: &str,
) -> Result<RunningState, String> {
    mutate_rewrite_job_session_now(app, state, session_id, |session, now| {
        session.status = compute_session_state(session);
        Ok(save_session_value(session, now, session.status))
    })
}

pub(crate) fn mark_chunks_running(
    app: &AppHandle,
    state: &AppState,
    session_id: &str,
    indices: &[usize],
) -> Result<(), String> {
    mutate_rewrite_job_session_now(app, state, session_id, |session, now| {
        set_chunks_running_status(session, indices)?;
        Ok(save_session_value(session, now, ()))
    })
}

pub(crate) fn mark_session_failed(
    app: &AppHandle,
    state: &AppState,
    session_id: &str,
    error: String,
) -> Result<(), String> {
    mutate_rewrite_job_session_now(app, state, session_id, |session, now| {
        session.status = RunningState::Failed;
        fail_running_chunks(session, &error);
        Ok(save_session_value(session, now, ()))
    })
}

pub(crate) fn mark_auto_batch_failed(
    app: &AppHandle,
    state: &AppState,
    session_id: &str,
    indices: &[usize],
    error: String,
) -> Result<(), String> {
    mutate_rewrite_job_session_now(app, state, session_id, |session, now| {
        fail_target_chunks_and_reset_other_running(session, indices, &error)?;
        Ok(save_session_value(session, now, ()))
    })
}
