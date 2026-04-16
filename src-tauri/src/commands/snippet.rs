use std::path::Path;

use tauri::{AppHandle, State};

use crate::{
    documents::{document_format, ensure_document_source_matches_session},
    editor_session::with_idle_editor_session,
    editor_writeback::ensure_session_can_use_plain_text_editor,
    models::{DocumentSession, DocumentSnapshot},
    rewrite,
    state::AppState,
    storage,
};

fn ensure_session_can_rewrite_snippet(session: &DocumentSession) -> Result<(), String> {
    ensure_session_can_use_plain_text_editor(session)?;
    ensure_document_source_matches_session(
        Path::new(&session.document_path),
        session.source_snapshot.as_ref(),
    )
}

#[tauri::command]
pub async fn rewrite_snippet(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    text: String,
    editor_base_snapshot: Option<DocumentSnapshot>,
) -> Result<String, String> {
    if text.trim().is_empty() {
        return Err("选区内容为空。".to_string());
    }

    let session = with_idle_editor_session(
        &app,
        &state,
        &session_id,
        &editor_base_snapshot,
        |session| {
            ensure_session_can_rewrite_snippet(&session)?;
            Ok(session)
        },
    )?;

    let settings = storage::load_settings(&app)?;
    let format = document_format(Path::new(&session.document_path));
    rewrite::rewrite_chunk(&settings, &text, format).await
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::ensure_session_can_rewrite_snippet;
    use crate::{
        document_snapshot::capture_document_snapshot,
        models::{ChunkStatus, ChunkTask, DocumentSession, EditSuggestion, SuggestionDecision},
        test_support::{cleanup_dir, sample_clean_session, write_temp_file},
    };

    fn sample_session() -> DocumentSession {
        let mut session = sample_clean_session("session-1", "/tmp/example.docx", "正文");
        session.chunks = vec![ChunkTask {
            index: 0,
            source_text: "正文".to_string(),
            separator_after: String::new(),
            skip_rewrite: false,
            presentation: None,
            status: ChunkStatus::Idle,
            error_message: None,
        }];
        session
    }

    #[test]
    fn rejects_snippet_rewrite_for_non_editor_safe_session() {
        let mut session = sample_session();
        session.plain_text_editor_safe = false;
        session.plain_text_editor_block_reason = Some("当前文档暂不支持进入编辑模式。".to_string());

        let error = ensure_session_can_rewrite_snippet(&session)
            .expect_err("expected snippet rewrite to be blocked");

        assert_eq!(error, "当前文档暂不支持进入编辑模式。");
    }

    #[test]
    fn rejects_snippet_rewrite_for_dirty_editor_session() {
        let (root, target) = write_temp_file("dirty-editor-session", "txt", "正文".as_bytes());

        let mut session = sample_session();
        session.document_path = target.to_string_lossy().to_string();
        session.source_text = "正文".to_string();
        session.source_snapshot =
            Some(capture_document_snapshot(&target).expect("capture initial snapshot"));
        session.suggestions.push(EditSuggestion {
            id: "s1".to_string(),
            sequence: 1,
            chunk_index: 0,
            before_text: "正文".to_string(),
            after_text: "改写正文".to_string(),
            diff_spans: Vec::new(),
            decision: SuggestionDecision::Proposed,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        });

        let error = ensure_session_can_rewrite_snippet(&session)
            .expect_err("expected dirty editor session to be blocked");

        assert!(error.contains("请先“覆写并清理记录”或“重置记录”后再编辑"));
        cleanup_dir(&root);
    }

    #[test]
    fn allows_snippet_rewrite_for_editor_safe_session() {
        let (root, target) = write_temp_file("source-match", "txt", "正文".as_bytes());

        let mut session = sample_session();
        session.document_path = target.to_string_lossy().to_string();
        session.source_text = "正文".to_string();
        session.source_snapshot =
            Some(capture_document_snapshot(&target).expect("capture initial snapshot"));

        ensure_session_can_rewrite_snippet(&session)
            .expect("expected snippet rewrite to be allowed");
        cleanup_dir(&root);
    }

    #[test]
    fn rejects_snippet_rewrite_when_source_changed_externally() {
        let (root, target) = write_temp_file("source-mismatch", "txt", "正文".as_bytes());

        let mut session = sample_session();
        session.document_path = target.to_string_lossy().to_string();
        session.source_text = "正文".to_string();
        session.source_snapshot =
            Some(capture_document_snapshot(&target).expect("capture initial snapshot"));

        fs::write(&target, "外部修改").expect("simulate external change");

        let error = ensure_session_can_rewrite_snippet(&session)
            .expect_err("expected snippet rewrite to be blocked");

        assert!(error.contains("原文件已在外部发生变化"));
        cleanup_dir(&root);
    }
}
