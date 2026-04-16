use std::fs;

use chrono::Utc;

use crate::{
    adapters::docx::DocxAdapter,
    document_snapshot::capture_document_snapshot,
    documents::{OwnedDocumentWriteback, RegionSegmentationStrategy, WritebackMode},
    models::{
        ChunkPreset, ChunkStatus, ChunkTask, DocumentSession, EditSuggestion, RunningState,
        SuggestionDecision,
    },
    rewrite,
    test_support::{build_minimal_docx, cleanup_dir, write_temp_file},
};

fn sample_plain_text_session(path: &std::path::Path) -> DocumentSession {
    let now = Utc::now();
    DocumentSession {
        id: "session-1".to_string(),
        title: "示例".to_string(),
        document_path: path.to_string_lossy().to_string(),
        source_text: "原文\r\n下一行\r\n".to_string(),
        source_snapshot: Some(capture_document_snapshot(path).expect("capture snapshot")),
        normalized_text: "原文\r\n下一行\r\n".to_string(),
        write_back_supported: true,
        write_back_block_reason: None,
        plain_text_editor_safe: true,
        plain_text_editor_block_reason: None,
        chunk_preset: Some(ChunkPreset::Paragraph),
        rewrite_headings: Some(false),
        chunks: vec![ChunkTask {
            index: 0,
            source_text: "原文\r\n下一行\r\n".to_string(),
            separator_after: String::new(),
            skip_rewrite: false,
            presentation: None,
            status: ChunkStatus::Idle,
            error_message: None,
        }],
        suggestions: vec![EditSuggestion {
            id: "suggestion-1".to_string(),
            sequence: 1,
            chunk_index: 0,
            before_text: "原文\r\n下一行\r\n".to_string(),
            after_text: "新文\n下一行  \n".to_string(),
            diff_spans: rewrite::build_diff("原文\r\n下一行\r\n", "新文\n下一行  \n"),
            decision: SuggestionDecision::Applied,
            created_at: now,
            updated_at: now,
        }],
        next_suggestion_sequence: 2,
        status: RunningState::Idle,
        created_at: now,
        updated_at: now,
    }
}

fn sample_preview_session() -> DocumentSession {
    let now = Utc::now();
    DocumentSession {
        id: "preview-session".to_string(),
        title: "示例".to_string(),
        document_path: "/tmp/example.txt".to_string(),
        source_text: "前文后文".to_string(),
        source_snapshot: None,
        normalized_text: "前文后文".to_string(),
        write_back_supported: true,
        write_back_block_reason: None,
        plain_text_editor_safe: true,
        plain_text_editor_block_reason: None,
        chunk_preset: Some(ChunkPreset::Paragraph),
        rewrite_headings: Some(false),
        chunks: vec![
            ChunkTask {
                index: 0,
                source_text: "前文".to_string(),
                separator_after: String::new(),
                skip_rewrite: false,
                presentation: None,
                status: ChunkStatus::Idle,
                error_message: None,
            },
            ChunkTask {
                index: 1,
                source_text: "后文".to_string(),
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

fn multi_paragraph_adjacent_styled_docx_session(
    path: &std::path::Path,
    bytes: &[u8],
) -> DocumentSession {
    let now = Utc::now();
    let regions = DocxAdapter::extract_regions(bytes, false).expect("extract regions");
    let source_text = regions
        .iter()
        .map(|region| region.body.as_str())
        .collect::<String>();
    let chunks = rewrite::segment_regions_with_strategy(
        regions,
        ChunkPreset::Paragraph,
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
        id: "session-docx-adjacent-styled".to_string(),
        title: "示例".to_string(),
        document_path: path.to_string_lossy().to_string(),
        source_text: source_text.clone(),
        source_snapshot: Some(capture_document_snapshot(path).expect("capture snapshot")),
        normalized_text: source_text,
        write_back_supported: true,
        write_back_block_reason: None,
        plain_text_editor_safe: true,
        plain_text_editor_block_reason: None,
        chunk_preset: Some(ChunkPreset::Paragraph),
        rewrite_headings: Some(false),
        chunks,
        suggestions: Vec::new(),
        next_suggestion_sequence: 1,
        status: RunningState::Idle,
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn build_session_writeback_plan_returns_plain_text_output() {
    let (root, target) = write_temp_file("session-plan", "txt", "原文\r\n下一行\r\n".as_bytes());
    let session = sample_plain_text_session(&target);

    match super::build_session_writeback_plan(&session) {
        OwnedDocumentWriteback::Text(text) => assert_eq!(text, "新文\n下一行  \n"),
        OwnedDocumentWriteback::Regions(_) => panic!("expected plain-text writeback plan"),
    }
    cleanup_dir(&root);
}

#[test]
fn execute_session_writeback_returns_block_error_before_loading_source() {
    let mut session = sample_preview_session();
    session.write_back_supported = false;
    session.write_back_block_reason = Some("blocked".to_string());
    session.suggestions.push(crate::models::EditSuggestion {
        id: "suggestion-1".to_string(),
        sequence: 1,
        chunk_index: 0,
        before_text: "前文".to_string(),
        after_text: "新前文".to_string(),
        diff_spans: rewrite::build_diff("前文", "新前文"),
        decision: SuggestionDecision::Applied,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    });

    let error = super::execute_session_writeback(&session, WritebackMode::Validate)
        .expect_err("expected blocked session to short-circuit before execute");

    assert_eq!(error, "blocked");
}

#[test]
fn write_final_session_document_normalizes_plain_text_output() {
    let (root, target) =
        write_temp_file("plain-text-write", "txt", "原文\r\n下一行\r\n".as_bytes());
    let session = sample_plain_text_session(&target);

    super::execute_session_writeback(&session, WritebackMode::Write)
        .expect("expected final session writeback to succeed");

    let stored = fs::read_to_string(&target).expect("read stored output");
    assert_eq!(stored, "新文\r\n下一行\r\n");
    cleanup_dir(&root);
}

#[test]
fn execute_session_writeback_validates_docx_with_adjacent_styled_regions_across_paragraphs() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:t>开头</w:t></w:r>
      <w:r><w:rPr><w:color w:val="FF0000"/></w:rPr><w:t>红色说明</w:t></w:r>
      <w:r><w:t>结尾说明</w:t></w:r>
    </w:p>
    <w:p><w:r><w:t>尾段</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
    let bytes = build_minimal_docx(xml);
    let (root, target) = write_temp_file("docx-adjacent-styled-validate", "docx", &bytes);
    let mut session = multi_paragraph_adjacent_styled_docx_session(&target, &bytes);
    let now = Utc::now();
    session.suggestions.push(EditSuggestion {
        id: "suggestion-1".to_string(),
        sequence: 1,
        chunk_index: 1,
        before_text: "红色说明".to_string(),
        after_text: "红色详细说明".to_string(),
        diff_spans: rewrite::build_diff("红色说明", "红色详细说明"),
        decision: SuggestionDecision::Applied,
        created_at: now,
        updated_at: now,
    });
    session.suggestions.push(EditSuggestion {
        id: "suggestion-2".to_string(),
        sequence: 2,
        chunk_index: 2,
        before_text: "结尾说明".to_string(),
        after_text: "结尾补充说明".to_string(),
        diff_spans: rewrite::build_diff("结尾说明", "结尾补充说明"),
        decision: SuggestionDecision::Applied,
        created_at: now,
        updated_at: now,
    });

    super::execute_session_writeback(&session, WritebackMode::Validate)
        .expect("expected adjacent styled regions across paragraphs to validate");

    cleanup_dir(&root);
}

#[test]
fn execute_session_writeback_validates_docx_when_adjacent_region_edits_share_one_diff_block() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:t>前</w:t></w:r>
      <w:r><w:rPr><w:color w:val="FF0000"/></w:rPr><w:t>BC</w:t></w:r>
      <w:r><w:t>DE</w:t></w:r>
    </w:p>
    <w:p><w:r><w:t>尾段</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
    let bytes = build_minimal_docx(xml);
    let (root, target) = write_temp_file("docx-adjacent-diff-block-validate", "docx", &bytes);
    let mut session = multi_paragraph_adjacent_styled_docx_session(&target, &bytes);
    let now = Utc::now();
    session.suggestions.push(EditSuggestion {
        id: "suggestion-1".to_string(),
        sequence: 1,
        chunk_index: 1,
        before_text: "BC".to_string(),
        after_text: "BX".to_string(),
        diff_spans: rewrite::build_diff("BC", "BX"),
        decision: SuggestionDecision::Applied,
        created_at: now,
        updated_at: now,
    });
    session.suggestions.push(EditSuggestion {
        id: "suggestion-2".to_string(),
        sequence: 2,
        chunk_index: 2,
        before_text: "DE".to_string(),
        after_text: "YE".to_string(),
        diff_spans: rewrite::build_diff("DE", "YE"),
        decision: SuggestionDecision::Applied,
        created_at: now,
        updated_at: now,
    });

    super::execute_session_writeback(&session, WritebackMode::Validate).expect(
        "expected adjacent editable regions to validate even when the combined diff crosses their boundary",
    );

    cleanup_dir(&root);
}
