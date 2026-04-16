use std::collections::HashMap;

use chrono::Utc;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::{
    documents::normalize_text_against_source_layout,
    models::{
        ChunkCompletedEvent, ChunkStatus, DocumentSession, EditSuggestion, RunningState,
        SuggestionDecision,
    },
    rewrite,
    rewrite_job_state::update_target_chunks,
    rewrite_permissions::CHUNK_INDEX_OUT_OF_RANGE_ERROR,
    rewrite_projection::apply_suggestion_by_id,
    session_access::CurrentSessionRequest,
    session_edit::{mutate_session_now, save_session_value},
    state::AppState,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BatchCommitMode {
    pub(crate) decision: SuggestionDecision,
    pub(crate) set_status: Option<RunningState>,
}

pub(crate) fn batch_commit_mode(auto_approve: bool) -> BatchCommitMode {
    if auto_approve {
        return BatchCommitMode {
            decision: SuggestionDecision::Applied,
            set_status: None,
        };
    }
    BatchCommitMode {
        decision: SuggestionDecision::Proposed,
        set_status: Some(RunningState::Idle),
    }
}

pub(crate) fn chunk_completed_events(
    session_id: &str,
    completed_batch: &[(usize, String, u64)],
) -> Vec<ChunkCompletedEvent> {
    completed_batch
        .iter()
        .map(
            |(index, suggestion_id, suggestion_sequence)| ChunkCompletedEvent {
                session_id: session_id.to_string(),
                index: *index,
                suggestion_id: suggestion_id.clone(),
                suggestion_sequence: *suggestion_sequence,
            },
        )
        .collect()
}

pub(crate) fn emit_chunk_completed_events(
    app: &AppHandle,
    session_id: &str,
    completed_batch: &[(usize, String, u64)],
) -> Result<(), String> {
    for event in chunk_completed_events(session_id, completed_batch) {
        app.emit("chunk_completed", event)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub(crate) fn commit_rewrite_result(
    app: &AppHandle,
    state: &AppState,
    session_id: &str,
    indices: &[usize],
    rewrite_result: Result<Vec<String>, String>,
    mode: BatchCommitMode,
    validate_batch_writeback: impl FnOnce(
        &DocumentSession,
        &HashMap<usize, String>,
    ) -> Result<(), String>,
) -> Result<Vec<(usize, String, u64)>, String> {
    match rewrite_result {
        Ok(candidate_texts) => commit_chunk_batch_success(
            app,
            state,
            session_id,
            indices,
            candidate_texts,
            mode,
            validate_batch_writeback,
        ),
        Err(error) => {
            commit_chunks_failure(app, state, session_id, indices, error.clone())?;
            Err(error)
        }
    }
}

fn commit_chunk_batch_success(
    app: &AppHandle,
    state: &AppState,
    session_id: &str,
    indices: &[usize],
    candidate_texts: Vec<String>,
    mode: BatchCommitMode,
    validate_batch_writeback: impl FnOnce(
        &DocumentSession,
        &HashMap<usize, String>,
    ) -> Result<(), String>,
) -> Result<Vec<(usize, String, u64)>, String> {
    mutate_session_now(
        CurrentSessionRequest::stored(app, state, session_id),
        |latest, now| {
            let mut normalized = normalize_candidate_batch(latest, indices, candidate_texts)?;
            validate_batch_writeback(latest, &normalized)?;
            let mut committed = Vec::with_capacity(indices.len());
            for index in indices.iter().copied() {
                let suggestion = create_committed_suggestion(latest, &mut normalized, index, now)?;
                apply_committed_suggestion(latest, index, suggestion.clone(), mode.decision)?;
                committed.push((index, suggestion.id, suggestion.sequence));
            }

            if let Some(status) = mode.set_status {
                latest.status = status;
            }

            Ok(save_session_value(latest, now, committed))
        },
    )
}

fn normalize_candidate_batch(
    session: &DocumentSession,
    indices: &[usize],
    candidate_texts: Vec<String>,
) -> Result<HashMap<usize, String>, String> {
    if indices.len() != candidate_texts.len() {
        return Err("批量改写结果数量与目标块数量不一致。".to_string());
    }

    let mut normalized = HashMap::with_capacity(indices.len());
    for (index, candidate_text) in indices.iter().copied().zip(candidate_texts.into_iter()) {
        let chunk_source_text = session
            .chunks
            .get(index)
            .ok_or_else(|| CHUNK_INDEX_OUT_OF_RANGE_ERROR.to_string())?
            .source_text
            .clone();
        normalized.insert(
            index,
            normalize_candidate_text(&chunk_source_text, &candidate_text),
        );
    }
    Ok(normalized)
}

fn create_committed_suggestion(
    session: &mut DocumentSession,
    normalized: &mut HashMap<usize, String>,
    index: usize,
    now: chrono::DateTime<Utc>,
) -> Result<EditSuggestion, String> {
    let chunk_source_text = session
        .chunks
        .get(index)
        .ok_or_else(|| CHUNK_INDEX_OUT_OF_RANGE_ERROR.to_string())?
        .source_text
        .clone();
    let candidate_text = normalized
        .remove(&index)
        .ok_or_else(|| "批量改写结果数量与目标块数量不一致。".to_string())?;
    let suggestion_id = Uuid::new_v4().to_string();
    let suggestion_sequence = session.next_suggestion_sequence;
    session.next_suggestion_sequence = session.next_suggestion_sequence.saturating_add(1);

    Ok(EditSuggestion {
        id: suggestion_id,
        sequence: suggestion_sequence,
        chunk_index: index,
        before_text: chunk_source_text.clone(),
        after_text: candidate_text.clone(),
        diff_spans: rewrite::build_diff(&chunk_source_text, &candidate_text),
        decision: SuggestionDecision::Applied,
        created_at: now,
        updated_at: now,
    })
}

fn apply_committed_suggestion(
    session: &mut DocumentSession,
    index: usize,
    suggestion: EditSuggestion,
    decision: SuggestionDecision,
) -> Result<(), String> {
    let now = suggestion.updated_at;
    let suggestion_id = suggestion.id.clone();
    let is_applied = decision == SuggestionDecision::Applied;
    let mut suggestion = suggestion;

    suggestion.decision = decision;
    session.suggestions.push(suggestion);
    if is_applied {
        apply_suggestion_by_id(session, &suggestion_id, now)?;
    }

    let chunk = session
        .chunks
        .get_mut(index)
        .ok_or_else(|| CHUNK_INDEX_OUT_OF_RANGE_ERROR.to_string())?;
    chunk.status = ChunkStatus::Done;
    chunk.error_message = None;
    Ok(())
}

fn commit_chunks_failure(
    app: &AppHandle,
    state: &AppState,
    session_id: &str,
    indices: &[usize],
    error: String,
) -> Result<(), String> {
    mutate_session_now(
        CurrentSessionRequest::stored(app, state, session_id),
        |latest, now| {
            update_target_chunks(latest, indices, ChunkStatus::Failed, Some(&error))?;
            latest.status = RunningState::Failed;
            Ok(save_session_value(latest, now, ()))
        },
    )
}

fn normalize_candidate_text(chunk_source_text: &str, candidate_text: &str) -> String {
    let mut normalized = normalize_text_against_source_layout(chunk_source_text, candidate_text);
    let source_has_line_break =
        chunk_source_text.contains('\n') || chunk_source_text.contains('\r');
    if !source_has_line_break {
        normalized = rewrite::collapse_line_breaks_to_spaces(&normalized);
    }
    normalized
}
