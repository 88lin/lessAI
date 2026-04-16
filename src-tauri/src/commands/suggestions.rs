use tauri::{AppHandle, State};

use crate::{
    documents::WritebackMode,
    models::{ChunkStatus, DocumentSession, SuggestionDecision},
    rewrite_projection::{
        apply_suggestion_by_id, find_suggestion_index, SUGGESTION_NOT_FOUND_ERROR,
    },
    rewrite_writeback::execute_session_writeback,
    session_access::CurrentSessionRequest,
    session_edit::mutate_session_cloned_now,
    state::AppState,
};

#[tauri::command]
pub fn apply_suggestion(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    suggestion_id: String,
) -> Result<DocumentSession, String> {
    mutate_session_cloned_now(
        CurrentSessionRequest::refreshed(&app, state.inner(), &session_id),
        |session, now| {
            apply_suggestion_by_id(session, &suggestion_id, now)?;
            execute_session_writeback(&session, WritebackMode::Validate)?;
            Ok(())
        },
    )
}

#[tauri::command]
pub fn dismiss_suggestion(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    suggestion_id: String,
) -> Result<DocumentSession, String> {
    mutate_session_cloned_now(
        CurrentSessionRequest::stored(&app, state.inner(), &session_id),
        |session, now| {
            let suggestion_index = find_suggestion_index(session, &suggestion_id)?;
            let suggestion = session
                .suggestions
                .get_mut(suggestion_index)
                .ok_or_else(|| SUGGESTION_NOT_FOUND_ERROR.to_string())?;

            suggestion.decision = SuggestionDecision::Dismissed;
            suggestion.updated_at = now;
            Ok(())
        },
    )
}

#[tauri::command]
pub fn delete_suggestion(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    suggestion_id: String,
) -> Result<DocumentSession, String> {
    mutate_session_cloned_now(
        CurrentSessionRequest::stored(&app, state.inner(), &session_id),
        |session, _| {
            let suggestion_index = find_suggestion_index(session, &suggestion_id)?;
            let removed = session
                .suggestions
                .get(suggestion_index)
                .ok_or_else(|| SUGGESTION_NOT_FOUND_ERROR.to_string())?
                .chunk_index;

            session.suggestions.retain(|item| item.id != suggestion_id);

            let still_has_any = session
                .suggestions
                .iter()
                .any(|item| item.chunk_index == removed);

            if !still_has_any {
                if let Some(chunk) = session.chunks.get_mut(removed) {
                    if chunk.status == ChunkStatus::Done {
                        chunk.status = ChunkStatus::Idle;
                    }
                }
            }

            Ok(())
        },
    )
}
