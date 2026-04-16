use tauri::{AppHandle, State};

use crate::{
    documents::WritebackMode,
    editor_session::{persist_rebuilt_editor_session, run_idle_editor_writeback},
    editor_writeback::{
        build_chunk_editor_writeback, build_plain_text_editor_writeback, EditorWritebackPayload,
    },
    models::{DocumentSession, DocumentSnapshot, EditorChunkEdit},
    state::AppState,
};

enum EditorCommandInput {
    Text(String),
    ChunkEdits(Vec<EditorChunkEdit>),
}

impl EditorCommandInput {
    fn build(self, session: &DocumentSession) -> Result<EditorWritebackPayload, String> {
        match self {
            Self::Text(content) => build_plain_text_editor_writeback(session, &content),
            Self::ChunkEdits(edits) => build_chunk_editor_writeback(session, &edits),
        }
    }
}

trait EditorCommandOutput: Sized {
    const MODE: WritebackMode;

    fn finish(app: &AppHandle, session: DocumentSession) -> Result<Self, String>;
}

impl EditorCommandOutput for () {
    const MODE: WritebackMode = WritebackMode::Validate;

    fn finish(_: &AppHandle, _: DocumentSession) -> Result<Self, String> {
        Ok(())
    }
}

impl EditorCommandOutput for DocumentSession {
    const MODE: WritebackMode = WritebackMode::Write;

    fn finish(app: &AppHandle, session: DocumentSession) -> Result<Self, String> {
        persist_rebuilt_editor_session(app, session)
    }
}

fn run_editor_command<T: EditorCommandOutput>(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    editor_base_snapshot: Option<DocumentSnapshot>,
    input: EditorCommandInput,
) -> Result<T, String> {
    run_idle_editor_writeback(
        &app,
        &state,
        &session_id,
        &editor_base_snapshot,
        T::MODE,
        move |session| input.build(session),
        |session| T::finish(&app, session),
    )
}

#[tauri::command]
pub fn validate_document_edits(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    content: String,
    editor_base_snapshot: Option<DocumentSnapshot>,
) -> Result<(), String> {
    run_editor_command(
        app,
        state,
        session_id,
        editor_base_snapshot,
        EditorCommandInput::Text(content),
    )
}

#[tauri::command]
pub fn validate_document_chunk_edits(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    edits: Vec<EditorChunkEdit>,
    editor_base_snapshot: Option<DocumentSnapshot>,
) -> Result<(), String> {
    run_editor_command(
        app,
        state,
        session_id,
        editor_base_snapshot,
        EditorCommandInput::ChunkEdits(edits),
    )
}

#[tauri::command]
pub fn save_document_edits(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    content: String,
    editor_base_snapshot: Option<DocumentSnapshot>,
) -> Result<DocumentSession, String> {
    run_editor_command(
        app,
        state,
        session_id,
        editor_base_snapshot,
        EditorCommandInput::Text(content),
    )
}

#[tauri::command]
pub fn save_document_chunk_edits(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    edits: Vec<EditorChunkEdit>,
    editor_base_snapshot: Option<DocumentSnapshot>,
) -> Result<DocumentSession, String> {
    run_editor_command(
        app,
        state,
        session_id,
        editor_base_snapshot,
        EditorCommandInput::ChunkEdits(edits),
    )
}
