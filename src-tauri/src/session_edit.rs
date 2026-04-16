use chrono::{DateTime, Utc};
use tauri::AppHandle;

use crate::{
    models::DocumentSession,
    persist,
    session_access::{access_current_session, CurrentSessionRequest},
    storage,
};

pub(crate) enum SessionMutation<T> {
    Save(T),
    SkipSave(T),
}

impl<T> SessionMutation<T> {
    fn into_parts(self) -> (T, bool) {
        match self {
            Self::Save(value) => (value, true),
            Self::SkipSave(value) => (value, false),
        }
    }
}

pub(crate) fn save_session_value<T>(
    session: &mut DocumentSession,
    updated_at: DateTime<Utc>,
    value: T,
) -> SessionMutation<T> {
    session.updated_at = updated_at;
    SessionMutation::Save(value)
}

pub(crate) fn save_cloned_session(
    session: &mut DocumentSession,
    updated_at: DateTime<Utc>,
) -> SessionMutation<DocumentSession> {
    session.updated_at = updated_at;
    SessionMutation::Save(session.clone())
}

pub(crate) fn mutate_session<T, Guard, Mutate>(
    request: CurrentSessionRequest<'_, Guard>,
    mutate: Mutate,
) -> Result<T, String>
where
    Guard: FnOnce(&DocumentSession) -> Result<(), String>,
    Mutate: FnOnce(&mut DocumentSession) -> Result<SessionMutation<T>, String>,
{
    let app = request.app;
    access_current_session(request, move |mut session| {
        persist_mutated_session(app, &mut session, mutate)
    })
}

pub(crate) fn mutate_session_now<T, Guard, Mutate>(
    request: CurrentSessionRequest<'_, Guard>,
    mutate: Mutate,
) -> Result<T, String>
where
    Guard: FnOnce(&DocumentSession) -> Result<(), String>,
    Mutate: FnOnce(&mut DocumentSession, DateTime<Utc>) -> Result<SessionMutation<T>, String>,
{
    mutate_session(request, |session| mutate(session, Utc::now()))
}

pub(crate) fn mutate_session_cloned_now<Guard, Mutate>(
    request: CurrentSessionRequest<'_, Guard>,
    mutate: Mutate,
) -> Result<DocumentSession, String>
where
    Guard: FnOnce(&DocumentSession) -> Result<(), String>,
    Mutate: FnOnce(&mut DocumentSession, DateTime<Utc>) -> Result<(), String>,
{
    mutate_session_now(request, |session, now| {
        mutate(session, now)?;
        Ok(save_cloned_session(session, now))
    })
}

fn persist_mutated_session<T, Mutate>(
    app: &AppHandle,
    session: &mut DocumentSession,
    mutate: Mutate,
) -> Result<T, String>
where
    Mutate: FnOnce(&mut DocumentSession) -> Result<SessionMutation<T>, String>,
{
    let (value, should_save) = mutate(session)?.into_parts();
    persist::maybe_save_and_return(value, should_save, |_| storage::save_session(app, session))
}

#[cfg(test)]
#[path = "session_edit_tests.rs"]
mod tests;
