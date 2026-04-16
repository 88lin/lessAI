use tauri::AppHandle;

use crate::{models::DocumentSession, state::AppState, storage};

use super::{load_rewriteable_session, next_manual_batch, process_loaded_chunk_batch};

pub(crate) async fn run_manual_rewrite(
    app: &AppHandle,
    state: &AppState,
    session_id: &str,
    target_chunk_indices: Option<Vec<usize>>,
) -> Result<DocumentSession, String> {
    let session = load_rewriteable_session(app, state, session_id)?;
    let settings = storage::load_settings(app)?;
    let next_batch =
        next_manual_batch(&session, target_chunk_indices, settings.chunks_per_request)?;

    process_loaded_chunk_batch(app, state, &session.id, &session, &next_batch, false).await?;
    crate::session_access::access_current_session(
        crate::session_access::CurrentSessionRequest::stored(app, state, &session.id),
        Ok,
    )
}
