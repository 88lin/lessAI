use std::{
    collections::{HashSet, VecDeque},
    path::Path,
    sync::atomic::Ordering,
};

use tauri::{AppHandle, Emitter};

use crate::{
    documents::document_format,
    models::{
        DocumentFormat, DocumentSession, RewriteMode, RewriteProgress, RunningState, SessionEvent,
    },
    rewrite_permissions::{
        ensure_session_can_rewrite, protected_chunk_rewrite_error, CHUNK_INDEX_OUT_OF_RANGE_ERROR,
    },
    rewrite_targets,
    session_access::{access_current_session, CurrentSessionRequest},
    state::{AppState, JobControl},
};

pub(super) const ACTIVE_REWRITE_SESSION_ERROR: &str = "当前文档正在执行自动任务，请先暂停或取消。";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RewriteSessionAccess {
    ExternalEntry,
    ActiveJob,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RewriteSourceChunk {
    skip_rewrite: bool,
    source_text: String,
}

pub(super) struct PreparedAutoRewriteSession {
    pub(super) format: DocumentFormat,
    pub(super) total_chunks: usize,
    pub(super) pending: VecDeque<usize>,
    pub(super) source_snapshot: Vec<RewriteSourceChunk>,
    pub(super) completed_chunks: usize,
}

pub(super) struct PreparedLoadedRewriteBatch {
    pub(super) format: DocumentFormat,
    pub(super) source_texts: Vec<String>,
}

pub(super) fn rewrite_session_request<'a>(
    app: &'a AppHandle,
    state: &'a AppState,
    session_id: &'a str,
) -> CurrentSessionRequest<'a, fn(&DocumentSession) -> Result<(), String>> {
    let request = CurrentSessionRequest::guarded_refresh(
        app,
        state,
        session_id,
        ensure_session_can_rewrite as fn(&DocumentSession) -> Result<(), String>,
    );
    match rewrite_session_active_job_error(RewriteSessionAccess::ExternalEntry) {
        Some(active_job_error) => request.with_active_job_error(active_job_error),
        None => request,
    }
}

fn active_job_rewrite_session_request<'a>(
    app: &'a AppHandle,
    state: &'a AppState,
    session_id: &'a str,
) -> CurrentSessionRequest<'a, fn(&DocumentSession) -> Result<(), String>> {
    let request = CurrentSessionRequest::guarded_refresh(
        app,
        state,
        session_id,
        ensure_session_can_rewrite as fn(&DocumentSession) -> Result<(), String>,
    );
    match rewrite_session_active_job_error(RewriteSessionAccess::ActiveJob) {
        Some(active_job_error) => request.with_active_job_error(active_job_error),
        None => request,
    }
}

pub(super) fn rewrite_session_active_job_error(
    access: RewriteSessionAccess,
) -> Option<&'static str> {
    match access {
        RewriteSessionAccess::ExternalEntry => Some(ACTIVE_REWRITE_SESSION_ERROR),
        RewriteSessionAccess::ActiveJob => None,
    }
}

pub(super) fn load_rewriteable_session(
    app: &AppHandle,
    state: &AppState,
    session_id: &str,
) -> Result<DocumentSession, String> {
    access_current_session(rewrite_session_request(app, state, session_id), Ok)
}

pub(super) fn load_rewriteable_session_for_active_job(
    app: &AppHandle,
    state: &AppState,
    session_id: &str,
) -> Result<DocumentSession, String> {
    access_current_session(
        active_job_rewrite_session_request(app, state, session_id),
        Ok,
    )
}

pub(super) fn build_rewrite_source_snapshot(session: &DocumentSession) -> Vec<RewriteSourceChunk> {
    session
        .chunks
        .iter()
        .map(|chunk| RewriteSourceChunk {
            skip_rewrite: chunk.skip_rewrite,
            source_text: chunk.source_text.clone(),
        })
        .collect()
}

pub(super) fn collect_rewrite_batch_source_texts(
    source_snapshot: &[RewriteSourceChunk],
    indices: &[usize],
) -> Result<Vec<String>, String> {
    indices
        .iter()
        .map(|index| {
            let chunk = source_snapshot
                .get(*index)
                .ok_or_else(|| CHUNK_INDEX_OUT_OF_RANGE_ERROR.to_string())?;
            if chunk.skip_rewrite {
                return Err(protected_chunk_rewrite_error(*index));
            }
            Ok(chunk.source_text.clone())
        })
        .collect()
}

pub(super) fn prepare_auto_rewrite_session(
    session: &DocumentSession,
    target_indices: Option<&HashSet<usize>>,
) -> PreparedAutoRewriteSession {
    PreparedAutoRewriteSession {
        format: document_format(Path::new(&session.document_path)),
        total_chunks: rewrite_targets::count_target_total_chunks(&session.chunks, target_indices),
        pending: rewrite_targets::build_auto_pending_queue(&session.chunks, target_indices),
        source_snapshot: build_rewrite_source_snapshot(session),
        completed_chunks: rewrite_targets::count_target_completed_chunks(
            &session.chunks,
            target_indices,
        ),
    }
}

pub(super) fn prepare_loaded_rewrite_batch(
    session: &DocumentSession,
    indices: &[usize],
) -> Result<PreparedLoadedRewriteBatch, String> {
    let source_snapshot = build_rewrite_source_snapshot(session);
    Ok(PreparedLoadedRewriteBatch {
        format: document_format(Path::new(&session.document_path)),
        source_texts: collect_rewrite_batch_source_texts(&source_snapshot, indices)?,
    })
}

pub(super) fn snapshot_running_indices_from_batches(
    in_flight_batches: &[Vec<usize>],
) -> Vec<usize> {
    let mut indices = in_flight_batches
        .iter()
        .flat_map(|batch| batch.iter().copied())
        .collect::<Vec<_>>();
    indices.sort_unstable();
    indices.dedup();
    indices
}

fn no_available_targets_error(has_target_subset: bool) -> String {
    if has_target_subset {
        "所选片段已处理完成。".to_string()
    } else {
        "没有可继续处理的片段，当前文档可能已经全部完成。".to_string()
    }
}

fn build_available_targets<T, Build, IsEmpty>(
    session: &DocumentSession,
    target_chunk_indices: Option<Vec<usize>>,
    build: Build,
    is_empty: IsEmpty,
) -> Result<T, String>
where
    Build: FnOnce(Option<&HashSet<usize>>) -> T,
    IsEmpty: FnOnce(&T) -> bool,
{
    let target_indices =
        rewrite_targets::resolve_target_indices(&session.chunks, target_chunk_indices)?;
    let targets = build(target_indices.as_ref());
    if is_empty(&targets) {
        return Err(no_available_targets_error(target_indices.is_some()));
    }
    Ok(targets)
}

pub(super) fn next_manual_batch(
    session: &DocumentSession,
    target_chunk_indices: Option<Vec<usize>>,
    batch_size: usize,
) -> Result<Vec<usize>, String> {
    build_available_targets(
        session,
        target_chunk_indices,
        |target_indices| {
            rewrite_targets::find_next_manual_batch(&session.chunks, target_indices, batch_size)
        },
        Vec::is_empty,
    )
}

pub(super) fn auto_pending_queue(
    session: &DocumentSession,
    target_chunk_indices: Option<Vec<usize>>,
) -> Result<Option<HashSet<usize>>, String> {
    build_available_targets(
        session,
        target_chunk_indices,
        |target_indices| {
            let pending =
                rewrite_targets::build_auto_pending_queue(&session.chunks, target_indices);
            (target_indices.cloned(), pending)
        },
        |(_, pending)| pending.is_empty(),
    )
    .map(|(target_indices, _)| target_indices)
}

pub(super) fn emit_rewrite_finished(app: &AppHandle, session_id: &str) -> Result<(), String> {
    app.emit(
        "rewrite_finished",
        SessionEvent {
            session_id: session_id.to_string(),
        },
    )
    .map_err(|error| error.to_string())
}

pub(super) fn emit_rewrite_progress(
    app: &AppHandle,
    session_id: &str,
    completed_chunks: usize,
    running_indices: Vec<usize>,
    total_chunks: usize,
    mode: RewriteMode,
    running_state: RunningState,
    max_concurrency: usize,
) -> Result<(), String> {
    let in_flight = running_indices.len();
    app.emit(
        "rewrite_progress",
        RewriteProgress {
            session_id: session_id.to_string(),
            completed_chunks,
            in_flight,
            running_indices,
            total_chunks,
            mode,
            running_state,
            max_concurrency,
        },
    )
    .map_err(|error| error.to_string())
}

pub(super) fn auto_running_state(job: &JobControl) -> RunningState {
    if job.paused.load(Ordering::SeqCst) {
        RunningState::Paused
    } else {
        RunningState::Running
    }
}

#[cfg(test)]
#[path = "support_tests.rs"]
mod tests;
