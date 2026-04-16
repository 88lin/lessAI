use std::path::Path;

use chrono::Utc;

use crate::{
    adapters::docx::DocxAdapter,
    document_snapshot::capture_document_snapshot,
    documents::{RegionSegmentationStrategy, WritebackMode},
    models::{
        ChunkStatus, ChunkTask, DocumentSession, EditSuggestion, RunningState, SuggestionDecision,
    },
    rewrite,
    rewrite_batch_commit::{batch_commit_mode, chunk_completed_events},
    rewrite_writeback::{execute_session_writeback, validate_candidate_batch_writeback},
    test_support::{build_minimal_docx, cleanup_dir, write_temp_file},
};

fn validate_candidate_writeback(
    session: &DocumentSession,
    index: usize,
    candidate_text: &str,
) -> Result<(), String> {
    validate_candidate_batch_writeback(
        session,
        &std::collections::HashMap::from([(index, candidate_text.to_string())]),
    )
}

fn sample_docx_session(path: &Path) -> DocumentSession {
    let now = Utc::now();
    DocumentSession {
        id: "session-1".to_string(),
        title: "示例".to_string(),
        document_path: path.to_string_lossy().to_string(),
        source_text: "正文".to_string(),
        source_snapshot: Some(capture_document_snapshot(path).expect("capture snapshot")),
        normalized_text: "正文".to_string(),
        write_back_supported: true,
        write_back_block_reason: None,
        plain_text_editor_safe: true,
        plain_text_editor_block_reason: None,
        chunk_preset: Some(crate::models::ChunkPreset::Paragraph),
        rewrite_headings: Some(false),
        chunks: vec![ChunkTask {
            index: 0,
            source_text: "正文".to_string(),
            separator_after: String::new(),
            skip_rewrite: false,
            presentation: None,
            status: ChunkStatus::Idle,
            error_message: None,
        }],
        suggestions: Vec::new(),
        next_suggestion_sequence: 1,
        status: RunningState::Idle,
        created_at: now,
        updated_at: now,
    }
}

fn collapsed_boundary_docx_session(path: &Path) -> DocumentSession {
    let now = Utc::now();
    DocumentSession {
        id: "session-collapsed-boundary".to_string(),
        title: "示例".to_string(),
        document_path: path.to_string_lossy().to_string(),
        source_text: "前文后文".to_string(),
        source_snapshot: Some(capture_document_snapshot(path).expect("capture snapshot")),
        normalized_text: "前文后文".to_string(),
        write_back_supported: true,
        write_back_block_reason: None,
        plain_text_editor_safe: true,
        plain_text_editor_block_reason: None,
        chunk_preset: Some(crate::models::ChunkPreset::Paragraph),
        rewrite_headings: Some(false),
        chunks: vec![ChunkTask {
            index: 0,
            source_text: "前文后文".to_string(),
            separator_after: String::new(),
            skip_rewrite: false,
            presentation: None,
            status: ChunkStatus::Idle,
            error_message: None,
        }],
        suggestions: Vec::new(),
        next_suggestion_sequence: 1,
        status: RunningState::Idle,
        created_at: now,
        updated_at: now,
    }
}

fn adjacent_styled_region_docx_session(path: &Path, bytes: &[u8]) -> DocumentSession {
    let now = Utc::now();
    let regions = DocxAdapter::extract_regions(bytes, false).expect("extract regions");
    let source_text = regions
        .iter()
        .map(|region| region.body.as_str())
        .collect::<String>();
    let chunks = rewrite::segment_regions_with_strategy(
        regions,
        crate::models::ChunkPreset::Paragraph,
        crate::models::DocumentFormat::PlainText,
        RegionSegmentationStrategy::PreserveBoundaries,
    )
    .into_iter()
    .enumerate()
    .map(|(index, chunk)| ChunkTask {
        index,
        source_text: chunk.text,
        separator_after: chunk.separator_after,
        skip_rewrite: chunk.skip_rewrite,
        presentation: chunk.presentation,
        status: ChunkStatus::Idle,
        error_message: None,
    })
    .collect::<Vec<_>>();
    DocumentSession {
        id: "session-adjacent-styled-regions".to_string(),
        title: "示例".to_string(),
        document_path: path.to_string_lossy().to_string(),
        source_text: source_text.clone(),
        source_snapshot: Some(capture_document_snapshot(path).expect("capture snapshot")),
        normalized_text: source_text,
        write_back_supported: true,
        write_back_block_reason: None,
        plain_text_editor_safe: true,
        plain_text_editor_block_reason: None,
        chunk_preset: Some(crate::models::ChunkPreset::Paragraph),
        rewrite_headings: Some(false),
        chunks,
        suggestions: Vec::new(),
        next_suggestion_sequence: 1,
        status: RunningState::Idle,
        created_at: now,
        updated_at: now,
    }
}

fn plain_text_session_with_protected_chunk(path: &Path) -> DocumentSession {
    let now = Utc::now();
    DocumentSession {
        id: "session-protected-plain-text".to_string(),
        title: "示例".to_string(),
        document_path: path.to_string_lossy().to_string(),
        source_text: "[公式]正文".to_string(),
        source_snapshot: Some(capture_document_snapshot(path).expect("capture snapshot")),
        normalized_text: "[公式]正文".to_string(),
        write_back_supported: true,
        write_back_block_reason: None,
        plain_text_editor_safe: true,
        plain_text_editor_block_reason: None,
        chunk_preset: Some(crate::models::ChunkPreset::Paragraph),
        rewrite_headings: Some(false),
        chunks: vec![
            ChunkTask {
                index: 0,
                source_text: "[公式]".to_string(),
                separator_after: String::new(),
                skip_rewrite: true,
                presentation: None,
                status: ChunkStatus::Done,
                error_message: None,
            },
            ChunkTask {
                index: 1,
                source_text: "正文".to_string(),
                separator_after: String::new(),
                skip_rewrite: false,
                presentation: None,
                status: ChunkStatus::Idle,
                error_message: None,
            },
        ],
        suggestions: Vec::new(),
        next_suggestion_sequence: 1,
        status: RunningState::Idle,
        created_at: now,
        updated_at: now,
    }
}

fn session_with_chunk_statuses(statuses: &[ChunkStatus]) -> DocumentSession {
    let now = Utc::now();
    DocumentSession {
        id: "session-statuses".to_string(),
        title: "示例".to_string(),
        document_path: "/tmp/example.txt".to_string(),
        source_text: "正文".to_string(),
        source_snapshot: None,
        normalized_text: "正文".to_string(),
        write_back_supported: true,
        write_back_block_reason: None,
        plain_text_editor_safe: true,
        plain_text_editor_block_reason: None,
        chunk_preset: Some(crate::models::ChunkPreset::Paragraph),
        rewrite_headings: Some(false),
        chunks: statuses
            .iter()
            .copied()
            .enumerate()
            .map(|(index, status)| ChunkTask {
                index,
                source_text: format!("chunk-{index}"),
                separator_after: String::new(),
                skip_rewrite: false,
                presentation: None,
                status,
                error_message: Some("旧错误".to_string()),
            })
            .collect(),
        suggestions: Vec::new(),
        next_suggestion_sequence: 1,
        status: RunningState::Idle,
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn clear_running_chunks_resets_only_running_chunks() {
    let mut session = session_with_chunk_statuses(&[
        ChunkStatus::Running,
        ChunkStatus::Failed,
        ChunkStatus::Done,
    ]);

    let touched = crate::rewrite_job_state::clear_running_chunks(&mut session);

    assert!(touched);
    assert_eq!(session.chunks[0].status, ChunkStatus::Idle);
    assert_eq!(session.chunks[0].error_message, None);
    assert_eq!(session.chunks[1].status, ChunkStatus::Failed);
    assert_eq!(session.chunks[1].error_message.as_deref(), Some("旧错误"));
    assert_eq!(session.chunks[2].status, ChunkStatus::Done);
    assert_eq!(session.chunks[2].error_message.as_deref(), Some("旧错误"));
}

#[test]
fn fail_running_chunks_marks_only_running_chunks() {
    let mut session =
        session_with_chunk_statuses(&[ChunkStatus::Running, ChunkStatus::Idle, ChunkStatus::Done]);

    let touched = crate::rewrite_job_state::fail_running_chunks(&mut session, "失败原因");

    assert!(touched);
    assert_eq!(session.chunks[0].status, ChunkStatus::Failed);
    assert_eq!(session.chunks[0].error_message.as_deref(), Some("失败原因"));
    assert_eq!(session.chunks[1].status, ChunkStatus::Idle);
    assert_eq!(session.chunks[1].error_message.as_deref(), Some("旧错误"));
    assert_eq!(session.chunks[2].status, ChunkStatus::Done);
    assert_eq!(session.chunks[2].error_message.as_deref(), Some("旧错误"));
}

#[test]
fn fail_target_chunks_and_reset_other_running_marks_only_failed_batch() {
    let mut session = session_with_chunk_statuses(&[
        ChunkStatus::Running,
        ChunkStatus::Running,
        ChunkStatus::Done,
    ]);
    session.status = RunningState::Running;

    crate::rewrite_job_state::fail_target_chunks_and_reset_other_running(
        &mut session,
        &[0],
        "写回校验失败",
    )
    .expect("expected targeted failure helper to succeed");

    assert_eq!(session.status, RunningState::Failed);
    assert_eq!(session.chunks[0].status, ChunkStatus::Failed);
    assert_eq!(
        session.chunks[0].error_message.as_deref(),
        Some("写回校验失败")
    );
    assert_eq!(session.chunks[1].status, ChunkStatus::Idle);
    assert_eq!(session.chunks[1].error_message, None);
    assert_eq!(session.chunks[2].status, ChunkStatus::Done);
    assert_eq!(session.chunks[2].error_message.as_deref(), Some("旧错误"));
}

#[test]
fn set_chunks_running_status_preserves_paused_session_state() {
    let mut session = session_with_chunk_statuses(&[ChunkStatus::Idle, ChunkStatus::Failed]);
    session.status = RunningState::Paused;

    crate::rewrite_job_state::set_chunks_running_status(&mut session, &[0])
        .expect("expected running-state helper to update chunk");

    assert_eq!(session.status, RunningState::Paused);
    assert_eq!(session.chunks[0].status, ChunkStatus::Running);
    assert_eq!(session.chunks[0].error_message, None);
    assert_eq!(session.chunks[1].status, ChunkStatus::Failed);
}

#[test]
fn set_session_cancelled_clears_running_chunks() {
    let mut session = session_with_chunk_statuses(&[
        ChunkStatus::Running,
        ChunkStatus::Failed,
        ChunkStatus::Done,
    ]);
    session.status = RunningState::Running;

    crate::rewrite_job_state::set_session_cancelled(&mut session);

    assert_eq!(session.status, RunningState::Cancelled);
    assert_eq!(session.chunks[0].status, ChunkStatus::Idle);
    assert_eq!(session.chunks[0].error_message, None);
    assert_eq!(session.chunks[1].status, ChunkStatus::Failed);
    assert_eq!(session.chunks[1].error_message.as_deref(), Some("旧错误"));
    assert_eq!(session.chunks[2].status, ChunkStatus::Done);
    assert_eq!(session.chunks[2].error_message.as_deref(), Some("旧错误"));
}

#[test]
fn set_session_paused_only_changes_session_status() {
    let mut session = session_with_chunk_statuses(&[
        ChunkStatus::Running,
        ChunkStatus::Failed,
        ChunkStatus::Done,
    ]);
    session.status = RunningState::Running;

    crate::rewrite_job_state::set_session_paused(&mut session);

    assert_eq!(session.status, RunningState::Paused);
    assert_eq!(session.chunks[0].status, ChunkStatus::Running);
    assert_eq!(session.chunks[0].error_message.as_deref(), Some("旧错误"));
    assert_eq!(session.chunks[1].status, ChunkStatus::Failed);
    assert_eq!(session.chunks[1].error_message.as_deref(), Some("旧错误"));
    assert_eq!(session.chunks[2].status, ChunkStatus::Done);
    assert_eq!(session.chunks[2].error_message.as_deref(), Some("旧错误"));
}

#[test]
fn set_session_running_only_changes_session_status() {
    let mut session = session_with_chunk_statuses(&[
        ChunkStatus::Running,
        ChunkStatus::Failed,
        ChunkStatus::Done,
    ]);
    session.status = RunningState::Paused;

    crate::rewrite_job_state::set_session_running(&mut session);

    assert_eq!(session.status, RunningState::Running);
    assert_eq!(session.chunks[0].status, ChunkStatus::Running);
    assert_eq!(session.chunks[0].error_message.as_deref(), Some("旧错误"));
    assert_eq!(session.chunks[1].status, ChunkStatus::Failed);
    assert_eq!(session.chunks[1].error_message.as_deref(), Some("旧错误"));
    assert_eq!(session.chunks[2].status, ChunkStatus::Done);
    assert_eq!(session.chunks[2].error_message.as_deref(), Some("旧错误"));
}

#[test]
fn apply_session_transition_steps_saves_snapshot_after_transition() {
    let mut session = session_with_chunk_statuses(&[
        ChunkStatus::Running,
        ChunkStatus::Failed,
        ChunkStatus::Done,
    ]);
    session.status = RunningState::Running;
    let original_updated_at = session.updated_at;
    let saved_at = original_updated_at + chrono::Duration::milliseconds(1);

    crate::rewrite_job_state::set_session_cancelled(&mut session);
    let mutation = crate::session_edit::save_cloned_session(&mut session, saved_at);

    match mutation {
        crate::session_edit::SessionMutation::Save(saved) => {
            assert_eq!(saved.status, RunningState::Cancelled);
            assert_eq!(saved.chunks[0].status, ChunkStatus::Idle);
            assert_eq!(saved.chunks[0].error_message, None);
            assert_eq!(saved.chunks[1].status, ChunkStatus::Failed);
            assert_eq!(saved.chunks[1].error_message.as_deref(), Some("旧错误"));
            assert!(saved.updated_at > original_updated_at);
        }
        crate::session_edit::SessionMutation::SkipSave(_) => {
            panic!("expected transition helper to request persist");
        }
    }

    assert_eq!(session.status, RunningState::Cancelled);
    assert_eq!(session.chunks[0].status, ChunkStatus::Idle);
    assert_eq!(session.chunks[0].error_message, None);
}

#[test]
fn update_target_chunks_rejects_out_of_range_without_partial_mutation() {
    let mut session =
        session_with_chunk_statuses(&[ChunkStatus::Idle, ChunkStatus::Failed, ChunkStatus::Done]);
    let original = session.chunks.clone();

    let error = crate::rewrite_job_state::update_target_chunks(
        &mut session,
        &[0, 99],
        ChunkStatus::Running,
        None,
    )
    .expect_err("expected invalid chunk indices to be rejected");

    assert_eq!(error, "片段索引越界。");
    assert_eq!(session.chunks.len(), original.len());
    for (current, previous) in session.chunks.iter().zip(original.iter()) {
        assert_eq!(current.index, previous.index);
        assert_eq!(current.source_text, previous.source_text);
        assert_eq!(current.separator_after, previous.separator_after);
        assert_eq!(current.skip_rewrite, previous.skip_rewrite);
        assert_eq!(current.presentation, previous.presentation);
        assert_eq!(current.status, previous.status);
        assert_eq!(current.error_message, previous.error_message);
    }
}

#[test]
fn save_session_value_marks_session_timestamp_and_requests_persist() {
    let mut session = session_with_chunk_statuses(&[ChunkStatus::Idle]);
    session.updated_at -= chrono::Duration::seconds(1);
    let original_updated_at = session.updated_at;

    let mutation =
        crate::session_edit::save_session_value(&mut session, chrono::Utc::now(), "saved");

    match mutation {
        crate::session_edit::SessionMutation::Save(value) => {
            assert_eq!(value, "saved");
        }
        crate::session_edit::SessionMutation::SkipSave(_) => {
            panic!("expected timestamped helper to request save");
        }
    }
    assert!(session.updated_at > original_updated_at);
}

#[test]
fn resolved_chunk_body_prefers_override_then_latest_applied_then_source() {
    let now = Utc::now();
    let chunk = ChunkTask {
        index: 0,
        source_text: "原文".to_string(),
        separator_after: String::new(),
        skip_rewrite: false,
        presentation: None,
        status: ChunkStatus::Idle,
        error_message: None,
    };
    let session = DocumentSession {
        id: "session-body-resolution".to_string(),
        title: "示例".to_string(),
        document_path: "/tmp/example.txt".to_string(),
        source_text: "原文".to_string(),
        source_snapshot: None,
        normalized_text: "原文".to_string(),
        write_back_supported: true,
        write_back_block_reason: None,
        plain_text_editor_safe: true,
        plain_text_editor_block_reason: None,
        chunk_preset: Some(crate::models::ChunkPreset::Paragraph),
        rewrite_headings: Some(false),
        chunks: vec![chunk.clone()],
        suggestions: vec![
            EditSuggestion {
                id: "dismissed".to_string(),
                sequence: 1,
                chunk_index: 0,
                before_text: "原文".to_string(),
                after_text: "旧改写".to_string(),
                diff_spans: Vec::new(),
                decision: SuggestionDecision::Dismissed,
                created_at: now,
                updated_at: now,
            },
            EditSuggestion {
                id: "applied-1".to_string(),
                sequence: 2,
                chunk_index: 0,
                before_text: "原文".to_string(),
                after_text: "已应用旧版".to_string(),
                diff_spans: Vec::new(),
                decision: SuggestionDecision::Applied,
                created_at: now,
                updated_at: now,
            },
            EditSuggestion {
                id: "applied-2".to_string(),
                sequence: 3,
                chunk_index: 0,
                before_text: "原文".to_string(),
                after_text: "已应用新版".to_string(),
                diff_spans: Vec::new(),
                decision: SuggestionDecision::Applied,
                created_at: now,
                updated_at: now,
            },
        ],
        next_suggestion_sequence: 4,
        status: RunningState::Idle,
        created_at: now,
        updated_at: now,
    };

    let no_override = crate::rewrite_projection::resolved_chunk_body(
        &session,
        &chunk,
        Some(&std::collections::HashMap::new()),
    );
    let mut overrides = std::collections::HashMap::new();
    overrides.insert(0, "覆盖文本".to_string());
    let with_override =
        crate::rewrite_projection::resolved_chunk_body(&session, &chunk, Some(&overrides));

    assert_eq!(no_override, "已应用新版");
    assert_eq!(with_override, "覆盖文本");
}

#[test]
fn apply_preview_suggestion_only_dismisses_same_chunk_applied_entries() {
    let now = Utc::now();
    let mut session = DocumentSession {
        id: "session-preview".to_string(),
        title: "示例".to_string(),
        document_path: "/tmp/example.txt".to_string(),
        source_text: "甲乙".to_string(),
        source_snapshot: None,
        normalized_text: "甲乙".to_string(),
        write_back_supported: true,
        write_back_block_reason: None,
        plain_text_editor_safe: true,
        plain_text_editor_block_reason: None,
        chunk_preset: Some(crate::models::ChunkPreset::Paragraph),
        rewrite_headings: Some(false),
        chunks: vec![
            ChunkTask {
                index: 0,
                source_text: "甲".to_string(),
                separator_after: String::new(),
                skip_rewrite: false,
                presentation: None,
                status: ChunkStatus::Idle,
                error_message: None,
            },
            ChunkTask {
                index: 1,
                source_text: "乙".to_string(),
                separator_after: String::new(),
                skip_rewrite: false,
                presentation: None,
                status: ChunkStatus::Idle,
                error_message: None,
            },
        ],
        suggestions: vec![
            EditSuggestion {
                id: "same-applied".to_string(),
                sequence: 1,
                chunk_index: 0,
                before_text: "甲".to_string(),
                after_text: "甲旧".to_string(),
                diff_spans: Vec::new(),
                decision: SuggestionDecision::Applied,
                created_at: now,
                updated_at: now,
            },
            EditSuggestion {
                id: "other-applied".to_string(),
                sequence: 2,
                chunk_index: 1,
                before_text: "乙".to_string(),
                after_text: "乙改".to_string(),
                diff_spans: Vec::new(),
                decision: SuggestionDecision::Applied,
                created_at: now,
                updated_at: now,
            },
        ],
        next_suggestion_sequence: 3,
        status: RunningState::Idle,
        created_at: now,
        updated_at: now,
    };

    crate::rewrite_projection::apply_preview_suggestion(&mut session, 0, "甲".to_string(), "甲新");

    let same = session
        .suggestions
        .iter()
        .find(|item| item.id == "same-applied")
        .expect("same-chunk suggestion");
    let other = session
        .suggestions
        .iter()
        .find(|item| item.id == "other-applied")
        .expect("other-chunk suggestion");
    let preview = session
        .suggestions
        .iter()
        .find(|item| item.id == "__preview__")
        .expect("preview suggestion");

    assert_eq!(same.decision, SuggestionDecision::Dismissed);
    assert_eq!(other.decision, SuggestionDecision::Applied);
    assert_eq!(preview.chunk_index, 0);
    assert_eq!(preview.after_text, "甲新");
    assert_eq!(preview.decision, SuggestionDecision::Applied);
}

#[test]
fn apply_suggestion_by_id_dismisses_same_chunk_applied_entries() {
    let now = Utc::now();
    let mut session = DocumentSession {
        id: "session-apply".to_string(),
        title: "示例".to_string(),
        document_path: "/tmp/example.txt".to_string(),
        source_text: "甲乙".to_string(),
        source_snapshot: None,
        normalized_text: "甲乙".to_string(),
        write_back_supported: true,
        write_back_block_reason: None,
        plain_text_editor_safe: true,
        plain_text_editor_block_reason: None,
        chunk_preset: Some(crate::models::ChunkPreset::Paragraph),
        rewrite_headings: Some(false),
        chunks: vec![
            ChunkTask {
                index: 0,
                source_text: "甲".to_string(),
                separator_after: String::new(),
                skip_rewrite: false,
                presentation: None,
                status: ChunkStatus::Idle,
                error_message: None,
            },
            ChunkTask {
                index: 1,
                source_text: "乙".to_string(),
                separator_after: String::new(),
                skip_rewrite: false,
                presentation: None,
                status: ChunkStatus::Idle,
                error_message: None,
            },
        ],
        suggestions: vec![
            EditSuggestion {
                id: "same-old".to_string(),
                sequence: 1,
                chunk_index: 0,
                before_text: "甲".to_string(),
                after_text: "甲旧".to_string(),
                diff_spans: Vec::new(),
                decision: SuggestionDecision::Applied,
                created_at: now,
                updated_at: now,
            },
            EditSuggestion {
                id: "same-new".to_string(),
                sequence: 2,
                chunk_index: 0,
                before_text: "甲".to_string(),
                after_text: "甲新".to_string(),
                diff_spans: Vec::new(),
                decision: SuggestionDecision::Proposed,
                created_at: now,
                updated_at: now,
            },
            EditSuggestion {
                id: "other-applied".to_string(),
                sequence: 3,
                chunk_index: 1,
                before_text: "乙".to_string(),
                after_text: "乙改".to_string(),
                diff_spans: Vec::new(),
                decision: SuggestionDecision::Applied,
                created_at: now,
                updated_at: now,
            },
        ],
        next_suggestion_sequence: 4,
        status: RunningState::Idle,
        created_at: now,
        updated_at: now,
    };

    crate::rewrite_projection::apply_suggestion_by_id(&mut session, "same-new", now)
        .expect("expected suggestion apply helper to succeed");

    let old = session
        .suggestions
        .iter()
        .find(|item| item.id == "same-old")
        .expect("same old");
    let new = session
        .suggestions
        .iter()
        .find(|item| item.id == "same-new")
        .expect("same new");
    let other = session
        .suggestions
        .iter()
        .find(|item| item.id == "other-applied")
        .expect("other");

    assert_eq!(old.decision, SuggestionDecision::Dismissed);
    assert_eq!(new.decision, SuggestionDecision::Applied);
    assert_eq!(other.decision, SuggestionDecision::Applied);
}

#[test]
fn apply_suggestion_by_id_rejects_missing_suggestion() {
    let now = Utc::now();
    let mut session = session_with_chunk_statuses(&[ChunkStatus::Idle]);

    let error = crate::rewrite_projection::apply_suggestion_by_id(&mut session, "missing", now)
        .expect_err("expected missing suggestion to be rejected");

    assert_eq!(error, "未找到对应的修改对。");
}

#[test]
fn batch_commit_mode_matches_auto_approve_flag() {
    let auto_mode = batch_commit_mode(true);
    let review_mode = batch_commit_mode(false);

    assert_eq!(auto_mode.decision, SuggestionDecision::Applied);
    assert_eq!(auto_mode.set_status, None);
    assert_eq!(review_mode.decision, SuggestionDecision::Proposed);
    assert_eq!(review_mode.set_status, Some(RunningState::Idle));
}

#[test]
fn with_rewrite_ready_session_returns_loaded_session_when_rewriteable() {
    let mut session = session_with_chunk_statuses(&[ChunkStatus::Idle]);
    session.document_path = "/tmp/example.pdf".to_string();
    let loaded = crate::session_flow::run_session_steps(
        || Ok(session.clone()),
        crate::session_flow::SessionStepConfig::new(crate::session_flow::allow_session),
        Ok,
    )
    .expect("expected rewriteable session to load");
    crate::rewrite_permissions::ensure_session_can_rewrite(&loaded)
        .expect("expected loaded session to stay rewriteable");

    assert_eq!(loaded.id, session.id);
}

#[test]
fn with_rewrite_ready_session_rejects_unrewriteable_session() {
    let mut session = session_with_chunk_statuses(&[ChunkStatus::Idle]);
    session.write_back_supported = false;
    session.write_back_block_reason = Some("blocked".to_string());
    let error = match crate::session_flow::run_session_steps(
        || Ok(session),
        crate::session_flow::SessionStepConfig::new(crate::session_flow::allow_session),
        Ok,
    ) {
        Ok(loaded) => match crate::rewrite_permissions::ensure_session_can_rewrite(&loaded) {
            Ok(_) => panic!("expected blocked session to be rejected"),
            Err(error) => error,
        },
        Err(error) => error,
    };

    assert_eq!(error, "blocked");
}

#[test]
fn next_manual_batch_returns_pending_indices_when_available() {
    let session =
        session_with_chunk_statuses(&[ChunkStatus::Done, ChunkStatus::Idle, ChunkStatus::Idle]);

    let batch = super::next_manual_batch(&session, None, 2)
        .expect("expected pending manual batch to resolve");

    assert_eq!(batch, vec![1, 2]);
}

#[test]
fn next_manual_batch_returns_selected_done_error_when_subset_is_exhausted() {
    let session =
        session_with_chunk_statuses(&[ChunkStatus::Done, ChunkStatus::Idle, ChunkStatus::Idle]);

    let error = match super::next_manual_batch(&session, Some(vec![0]), 1) {
        Ok(_) => panic!("expected exhausted selected subset to be rejected"),
        Err(error) => error,
    };

    assert_eq!(error, "所选片段已处理完成。");
}

#[test]
fn auto_pending_queue_returns_global_done_error_when_document_is_exhausted() {
    let session =
        session_with_chunk_statuses(&[ChunkStatus::Done, ChunkStatus::Done, ChunkStatus::Done]);

    let error = match super::auto_pending_queue(&session, None) {
        Ok(_) => panic!("expected exhausted document to be rejected"),
        Err(error) => error,
    };

    assert_eq!(error, "没有可继续处理的片段，当前文档可能已经全部完成。");
}

#[test]
fn collect_rewrite_batch_source_texts_returns_texts_in_batch_order() {
    let session =
        session_with_chunk_statuses(&[ChunkStatus::Done, ChunkStatus::Idle, ChunkStatus::Idle]);
    let snapshot = super::build_rewrite_source_snapshot(&session);

    let texts = super::collect_rewrite_batch_source_texts(&snapshot, &[2, 1])
        .expect("expected shared batch source collector to preserve request order");

    assert_eq!(texts, vec!["chunk-2".to_string(), "chunk-1".to_string()]);
}

#[test]
fn collect_rewrite_batch_source_texts_rejects_protected_chunk() {
    let mut session = session_with_chunk_statuses(&[ChunkStatus::Idle, ChunkStatus::Idle]);
    session.chunks[1].skip_rewrite = true;
    let snapshot = super::build_rewrite_source_snapshot(&session);

    let error = super::collect_rewrite_batch_source_texts(&snapshot, &[1])
        .expect_err("expected shared batch source collector to reject protected chunk");

    assert_eq!(error, "第 2 段属于保护区，不允许 AI 改写。");
}

#[test]
fn chunk_completed_events_preserve_batch_order_and_session_id() {
    let events = chunk_completed_events(
        "session-123",
        &[
            (2, "suggestion-b".to_string(), 11),
            (0, "suggestion-a".to_string(), 9),
        ],
    );

    assert_eq!(events.len(), 2);
    assert_eq!(events[0].session_id, "session-123");
    assert_eq!(events[0].index, 2);
    assert_eq!(events[0].suggestion_id, "suggestion-b");
    assert_eq!(events[0].suggestion_sequence, 11);
    assert_eq!(events[1].session_id, "session-123");
    assert_eq!(events[1].index, 0);
    assert_eq!(events[1].suggestion_id, "suggestion-a");
    assert_eq!(events[1].suggestion_sequence, 9);
}

#[test]
fn validate_candidate_writeback_rejects_docx_candidate_that_changes_paragraph_boundaries() {
    let document_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>正文</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
    let bytes = build_minimal_docx(document_xml);
    let (root, target) = write_temp_file("candidate-validate-fail", "docx", &bytes);
    let session = sample_docx_session(&target);

    let error = validate_candidate_writeback(&session, 0, "正文\n\n新增段")
        .expect_err("expected candidate validation failure");

    assert!(
        error.contains("段落")
            || error.contains("空段落边界")
            || error.contains("写回内容与原 docx 结构不一致")
    );
    cleanup_dir(&root);
}

#[test]
fn validate_session_writeback_rejects_unwritable_docx_applied_suggestion() {
    let document_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>正文</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
    let bytes = build_minimal_docx(document_xml);
    let (root, target) = write_temp_file("session-validate-fail", "docx", &bytes);
    let mut session = sample_docx_session(&target);
    let now = Utc::now();
    session.suggestions.push(EditSuggestion {
        id: "suggestion-1".to_string(),
        sequence: 1,
        chunk_index: 0,
        before_text: "正文".to_string(),
        after_text: "正文\n\n新增段".to_string(),
        diff_spans: Vec::new(),
        decision: SuggestionDecision::Applied,
        created_at: now,
        updated_at: now,
    });

    let error = execute_session_writeback(&session, WritebackMode::Validate)
        .expect_err("expected applied validation failure");

    assert!(
        error.contains("段落")
            || error.contains("空段落边界")
            || error.contains("写回内容与原 docx 结构不一致")
    );
    cleanup_dir(&root);
}

#[test]
fn validate_candidate_writeback_rejects_stale_docx_session_with_collapsed_boundaries() {
    let document_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:t>前文</w:t></w:r>
      <w:r><w:rPr><w:u w:val="single"/></w:rPr><w:t>后文</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
    let bytes = build_minimal_docx(document_xml);
    let (root, target) = write_temp_file("candidate-projection-pass", "docx", &bytes);
    let session = collapsed_boundary_docx_session(&target);

    let error = validate_candidate_writeback(&session, 0, "前文新文")
        .expect_err("expected stale collapsed-boundary session to be rejected");

    assert!(
        error.contains("区域数量不足") || error.contains("边界") || error.contains("结构不一致")
    );

    cleanup_dir(&root);
}

#[test]
fn validate_session_writeback_rejects_stale_docx_session_with_collapsed_boundaries() {
    let document_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:t>前文</w:t></w:r>
      <w:r><w:rPr><w:u w:val="single"/></w:rPr><w:t>后文</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
    let bytes = build_minimal_docx(document_xml);
    let (root, target) = write_temp_file("session-projection-pass", "docx", &bytes);
    let mut session = collapsed_boundary_docx_session(&target);
    let now = Utc::now();
    session.suggestions.push(EditSuggestion {
        id: "suggestion-1".to_string(),
        sequence: 1,
        chunk_index: 0,
        before_text: "前文后文".to_string(),
        after_text: "前文新文".to_string(),
        diff_spans: Vec::new(),
        decision: SuggestionDecision::Applied,
        created_at: now,
        updated_at: now,
    });

    let error = execute_session_writeback(&session, WritebackMode::Validate)
        .expect_err("expected stale collapsed-boundary session to be rejected");

    assert!(
        error.contains("区域数量不足") || error.contains("边界") || error.contains("结构不一致")
    );

    cleanup_dir(&root);
}

#[test]
fn validate_candidate_writeback_allows_edit_at_start_of_adjacent_styled_region() {
    let document_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:t>前文</w:t></w:r>
      <w:r><w:rPr><w:u w:val="single"/></w:rPr><w:t>后文</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
    let bytes = build_minimal_docx(document_xml);
    let (root, target) = write_temp_file("candidate-adjacent-styled-boundary-pass", "docx", &bytes);
    let session = adjacent_styled_region_docx_session(&target, &bytes);

    validate_candidate_writeback(&session, 1, "新后文")
        .expect("expected candidate validation to preserve known chunk boundary");

    cleanup_dir(&root);
}

#[test]
fn validate_session_writeback_allows_applied_edit_at_start_of_adjacent_styled_region() {
    let document_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:t>前文</w:t></w:r>
      <w:r><w:rPr><w:u w:val="single"/></w:rPr><w:t>后文</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
    let bytes = build_minimal_docx(document_xml);
    let (root, target) = write_temp_file("session-adjacent-styled-boundary-pass", "docx", &bytes);
    let mut session = adjacent_styled_region_docx_session(&target, &bytes);
    let now = Utc::now();
    session.suggestions.push(EditSuggestion {
        id: "suggestion-1".to_string(),
        sequence: 1,
        chunk_index: 1,
        before_text: "后文".to_string(),
        after_text: "新后文".to_string(),
        diff_spans: Vec::new(),
        decision: SuggestionDecision::Applied,
        created_at: now,
        updated_at: now,
    });

    execute_session_writeback(&session, WritebackMode::Validate)
        .expect("expected session validation to preserve known chunk boundary");

    cleanup_dir(&root);
}

#[test]
fn validate_candidate_writeback_rejects_protected_plain_text_chunk() {
    let (root, target) = write_temp_file(
        "candidate-protected-plain-text",
        "txt",
        "[公式]正文".as_bytes(),
    );
    let session = plain_text_session_with_protected_chunk(&target);

    let error = validate_candidate_writeback(&session, 0, "改公式")
        .expect_err("expected protected chunk candidate to be rejected");

    assert!(error.contains("保护区") || error.contains("不可改写"));
    cleanup_dir(&root);
}

#[test]
fn validate_session_writeback_rejects_applied_suggestion_on_protected_plain_text_chunk() {
    let (root, target) = write_temp_file(
        "session-protected-plain-text",
        "txt",
        "[公式]正文".as_bytes(),
    );
    let mut session = plain_text_session_with_protected_chunk(&target);
    let now = Utc::now();
    session.suggestions.push(EditSuggestion {
        id: "suggestion-protected".to_string(),
        sequence: 1,
        chunk_index: 0,
        before_text: "[公式]".to_string(),
        after_text: "改公式".to_string(),
        diff_spans: Vec::new(),
        decision: SuggestionDecision::Applied,
        created_at: now,
        updated_at: now,
    });

    let error = execute_session_writeback(&session, WritebackMode::Validate)
        .expect_err("expected applied protected-chunk suggestion to be rejected");

    assert!(error.contains("保护区") || error.contains("不可改写"));
    cleanup_dir(&root);
}
