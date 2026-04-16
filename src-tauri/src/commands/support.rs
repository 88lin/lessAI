use tauri::{AppHandle, State};

use crate::{
    models::DocumentSession,
    session_access::{access_current_session, CurrentSessionRequest},
    state::AppState,
};

pub(super) enum SessionCommandSource {
    Stored,
    Refreshed,
}

pub(super) fn run_session_command<T, Run>(
    app: &AppHandle,
    state: &State<'_, AppState>,
    session_id: &str,
    source: SessionCommandSource,
    active_job_error: Option<&str>,
    run: Run,
) -> Result<T, String>
where
    Run: FnOnce(DocumentSession) -> Result<T, String>,
{
    let request = match source {
        SessionCommandSource::Stored => {
            CurrentSessionRequest::stored(app, state.inner(), session_id)
        }
        SessionCommandSource::Refreshed => {
            CurrentSessionRequest::refreshed(app, state.inner(), session_id)
        }
    };
    let request = match active_job_error {
        Some(active_job_error) => request.with_active_job_error(active_job_error),
        None => request,
    };
    access_current_session(request, run)
}
