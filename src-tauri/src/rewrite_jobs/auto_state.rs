use tauri::AppHandle;

use crate::{
    rewrite_batch_commit::{batch_commit_mode, commit_rewrite_result, emit_chunk_completed_events},
    rewrite_job_state::{mark_auto_batch_failed, mark_session_cancelled, mark_session_failed},
    rewrite_writeback::validate_candidate_batch_writeback,
    state::AppState,
};

pub(super) type AutoTaskJoin = (Vec<usize>, Result<Vec<String>, String>);

pub(super) const UNKNOWN_IN_FLIGHT_BATCH_ERROR: &str =
    "自动改写任务状态异常：收到未登记批次的完成结果。";
pub(super) const TASK_SET_DRAINED_WITH_IN_FLIGHT_BATCHES_ERROR: &str =
    "自动改写任务状态异常：后台任务集合已清空，但仍存在未完成批次。";

pub(super) enum AutoLoopStop<'a> {
    Cancelled,
    SessionFailed(String),
    BatchFailed { indices: &'a [usize], error: String },
}

pub(super) fn commit_auto_batch(
    app: &AppHandle,
    state: &AppState,
    session_id: &str,
    indices: &[usize],
    candidate_texts: Vec<String>,
) -> Result<Vec<(usize, String, u64)>, String> {
    let completed_batch = commit_rewrite_result(
        app,
        state,
        session_id,
        indices,
        Ok(candidate_texts),
        batch_commit_mode(true),
        validate_candidate_batch_writeback,
    )?;
    emit_chunk_completed_events(app, session_id, &completed_batch)?;
    Ok(completed_batch)
}

pub(super) fn finish_auto_loop(
    app: &AppHandle,
    state: &AppState,
    session_id: &str,
    tasks: &mut tokio::task::JoinSet<AutoTaskJoin>,
    in_flight_batches: &mut Vec<Vec<usize>>,
    stop: AutoLoopStop<'_>,
) -> Result<(), String> {
    abort_in_flight(tasks, in_flight_batches);
    match stop {
        AutoLoopStop::Cancelled => {
            mark_session_cancelled(app, state, session_id)?;
            Ok(())
        }
        AutoLoopStop::SessionFailed(error) => {
            mark_session_failed(app, state, session_id, error.clone())?;
            Err(error)
        }
        AutoLoopStop::BatchFailed { indices, error } => {
            mark_auto_batch_failed(app, state, session_id, indices, error.clone())?;
            Err(error)
        }
    }
}

pub(super) fn ensure_in_flight_batches_drained(
    in_flight_batches: &[Vec<usize>],
) -> Result<(), String> {
    if in_flight_batches.is_empty() {
        return Ok(());
    }
    Err(TASK_SET_DRAINED_WITH_IN_FLIGHT_BATCHES_ERROR.to_string())
}

pub(super) fn remove_in_flight_batch(
    in_flight_batches: &mut Vec<Vec<usize>>,
    indices: &[usize],
) -> Result<(), String> {
    let Some(position) = in_flight_batches.iter().position(|batch| batch == indices) else {
        return Err(UNKNOWN_IN_FLIGHT_BATCH_ERROR.to_string());
    };
    in_flight_batches.remove(position);
    Ok(())
}

pub(super) fn abort_in_flight(
    tasks: &mut tokio::task::JoinSet<AutoTaskJoin>,
    in_flight_batches: &mut Vec<Vec<usize>>,
) {
    tasks.abort_all();
    in_flight_batches.clear();
}
