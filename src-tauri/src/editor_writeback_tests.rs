use chrono::Utc;
use std::cell::{Cell, RefCell};

use crate::{
    models::{ChunkPresentation, ChunkStatus, ChunkTask, DocumentSession, RunningState},
    rewrite_projection::merged_text_from_regions,
};

use super::{
    build_chunk_editor_writeback, build_plain_text_editor_writeback,
    build_updated_regions_from_chunk_edits, EditorWritebackPayload,
};

fn sample_session(chunks: Vec<ChunkTask>) -> DocumentSession {
    let now = Utc::now();
    DocumentSession {
        id: "session-1".to_string(),
        title: "示例".to_string(),
        document_path: "/tmp/example.docx".to_string(),
        source_text: chunks
            .iter()
            .map(|chunk| format!("{}{}", chunk.source_text, chunk.separator_after))
            .collect::<String>(),
        source_snapshot: None,
        normalized_text: String::new(),
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

fn editable_chunk(index: usize, text: &str, presentation: Option<ChunkPresentation>) -> ChunkTask {
    ChunkTask {
        index,
        source_text: text.to_string(),
        separator_after: String::new(),
        skip_rewrite: false,
        presentation,
        status: ChunkStatus::Idle,
        error_message: None,
    }
}

fn locked_chunk(index: usize, text: &str, protect_kind: &str) -> ChunkTask {
    ChunkTask {
        index,
        source_text: text.to_string(),
        separator_after: String::new(),
        skip_rewrite: true,
        presentation: Some(ChunkPresentation {
            bold: false,
            italic: false,
            underline: false,
            href: None,
            protect_kind: Some(protect_kind.to_string()),
            writeback_key: None,
        }),
        status: ChunkStatus::Done,
        error_message: None,
    }
}

#[test]
fn with_chunk_edit_overrides_loads_overrides_then_runs() {
    let session = sample_session(vec![
        editable_chunk(0, "前文", None),
        locked_chunk(1, "[公式]", "formula"),
        editable_chunk(2, "后文", None),
    ]);
    let edits = vec![
        crate::models::EditorChunkEdit {
            index: 0,
            text: "新前文".to_string(),
        },
        crate::models::EditorChunkEdit {
            index: 2,
            text: "新后文".to_string(),
        },
    ];
    let calls = RefCell::new(Vec::new());

    let text = super::with_chunk_edit_overrides(
        &session,
        &edits,
        |session, overrides: &std::collections::HashMap<usize, String>| {
            calls.borrow_mut().push(format!("run:{}", session.id));
            Ok(session
                .chunks
                .iter()
                .map(|chunk| {
                    overrides
                        .get(&chunk.index)
                        .cloned()
                        .unwrap_or_else(|| chunk.source_text.clone())
                })
                .collect::<String>())
        },
    )
    .expect("expected helper to collect overrides then run");

    assert_eq!(text, "新前文[公式]新后文");
    assert_eq!(calls.into_inner(), vec!["run:session-1".to_string()]);
}

#[test]
fn with_chunk_edit_overrides_stops_before_run_when_override_collection_fails() {
    let session = sample_session(vec![
        editable_chunk(0, "第一段", None),
        editable_chunk(1, "第二段", None),
    ]);
    let edits = vec![crate::models::EditorChunkEdit {
        index: 0,
        text: "改第一段".to_string(),
    }];
    let run_calls = Cell::new(0);

    let error = super::with_chunk_edit_overrides(&session, &edits, |_, _| {
        run_calls.set(run_calls.get() + 1);
        Ok(())
    })
    .expect_err("expected override collection failure to short-circuit");

    assert!(error.contains("数量") || error.contains("可编辑"));
    assert_eq!(run_calls.get(), 0);
}

#[test]
fn builds_text_from_chunk_edits_with_locked_content_between_editable_chunks() {
    let session = sample_session(vec![
        editable_chunk(0, "前文", None),
        locked_chunk(1, "[公式]", "formula"),
        editable_chunk(2, "后文", None),
    ]);
    let edits = vec![
        crate::models::EditorChunkEdit {
            index: 0,
            text: "新前文".to_string(),
        },
        crate::models::EditorChunkEdit {
            index: 2,
            text: "新后文".to_string(),
        },
    ];

    let text = merged_text_from_regions(
        &build_updated_regions_from_chunk_edits(&session, &edits).expect("expected chunk edits"),
    );

    assert_eq!(text, "新前文[公式]新后文");
}

#[test]
fn rejects_chunk_edit_payload_when_any_editable_chunk_is_missing() {
    let session = sample_session(vec![
        editable_chunk(0, "第一段", None),
        editable_chunk(1, "第二段", None),
    ]);
    let edits = vec![crate::models::EditorChunkEdit {
        index: 0,
        text: "改第一段".to_string(),
    }];

    let error = build_updated_regions_from_chunk_edits(&session, &edits)
        .expect_err("expected missing editable chunk to be rejected");

    assert!(error.contains("数量") || error.contains("可编辑"));
}

#[test]
fn keeps_adjacent_editable_chunks_text_when_presentations_differ() {
    let bold = Some(ChunkPresentation {
        bold: true,
        italic: false,
        underline: false,
        href: None,
        protect_kind: None,
        writeback_key: None,
    });
    let plain = Some(ChunkPresentation {
        bold: false,
        italic: false,
        underline: false,
        href: None,
        protect_kind: None,
        writeback_key: None,
    });
    let session = sample_session(vec![
        editable_chunk(0, "加粗", bold.clone()),
        editable_chunk(1, "正文", plain.clone()),
    ]);
    let edits = vec![
        crate::models::EditorChunkEdit {
            index: 0,
            text: "粗体".to_string(),
        },
        crate::models::EditorChunkEdit {
            index: 1,
            text: "内容".to_string(),
        },
    ];

    let text = merged_text_from_regions(
        &build_updated_regions_from_chunk_edits(&session, &edits)
            .expect("expected adjacent editable chunks to stay writeback-safe"),
    );

    assert_eq!(text, "粗体内容");
    assert!(bold.is_some());
    assert!(plain.is_some());
}

#[test]
fn preserves_empty_editable_region_text_when_presentation_boundary_matters() {
    let bold = Some(ChunkPresentation {
        bold: true,
        italic: false,
        underline: false,
        href: None,
        protect_kind: None,
        writeback_key: Some("r:bold".to_string()),
    });
    let plain = Some(ChunkPresentation {
        bold: false,
        italic: false,
        underline: false,
        href: None,
        protect_kind: None,
        writeback_key: None,
    });
    let session = sample_session(vec![
        editable_chunk(0, "标题", bold.clone()),
        editable_chunk(1, "正文", plain.clone()),
    ]);
    let edits = vec![
        crate::models::EditorChunkEdit {
            index: 0,
            text: String::new(),
        },
        crate::models::EditorChunkEdit {
            index: 1,
            text: "保留正文".to_string(),
        },
    ];

    let text = merged_text_from_regions(
        &build_updated_regions_from_chunk_edits(&session, &edits)
            .expect("expected empty editable region text to stay buildable"),
    );

    assert_eq!(text, "保留正文");
    assert!(bold.is_some());
    assert!(plain.is_some());
}

#[test]
fn builds_regions_from_chunk_edits_preserving_adjacent_presentation_boundaries() {
    let bold = Some(ChunkPresentation {
        bold: true,
        italic: false,
        underline: false,
        href: None,
        protect_kind: None,
        writeback_key: Some("r:bold".to_string()),
    });
    let underline = Some(ChunkPresentation {
        bold: false,
        italic: false,
        underline: true,
        href: None,
        protect_kind: None,
        writeback_key: Some("r:u".to_string()),
    });
    let session = sample_session(vec![
        editable_chunk(0, "前文", bold.clone()),
        editable_chunk(1, "后文", underline.clone()),
    ]);
    let edits = vec![
        crate::models::EditorChunkEdit {
            index: 0,
            text: "新前文".to_string(),
        },
        crate::models::EditorChunkEdit {
            index: 1,
            text: "新后文".to_string(),
        },
    ];

    let regions = build_updated_regions_from_chunk_edits(&session, &edits)
        .expect("expected chunk edits to preserve region boundaries");

    assert_eq!(regions.len(), 2);
    assert_eq!(regions[0].body, "新前文");
    assert_eq!(regions[1].body, "新后文");
    assert_eq!(regions[0].presentation, bold);
    assert_eq!(regions[1].presentation, underline);
}

#[test]
fn build_plain_text_editor_writeback_normalizes_non_docx_content() {
    let mut session = sample_session(vec![editable_chunk(0, "原文\r\n下一行\r\n", None)]);
    session.document_path = "/tmp/example.txt".to_string();
    session.source_text = "原文\r\n下一行\r\n".to_string();

    let payload = build_plain_text_editor_writeback(&session, "新文\n下一行  \n")
        .expect("expected plain text editor payload");

    match payload {
        EditorWritebackPayload::Text(text) => assert_eq!(text, "新文\r\n下一行\r\n"),
        EditorWritebackPayload::Regions(_) => panic!("expected plain text payload"),
    }
}

#[test]
fn build_plain_text_editor_writeback_rejects_empty_content() {
    let mut session = sample_session(vec![editable_chunk(0, "原文", None)]);
    session.document_path = "/tmp/example.txt".to_string();
    session.source_text = "原文".to_string();

    let error = match build_plain_text_editor_writeback(&session, "   ") {
        Ok(_) => panic!("expected empty plain text editor content to be rejected"),
        Err(error) => error,
    };

    assert_eq!(error, "文档内容为空，无法保存。");
}

#[test]
fn build_chunk_editor_writeback_builds_docx_regions_from_edits() {
    let bold = Some(ChunkPresentation {
        bold: true,
        italic: false,
        underline: false,
        href: None,
        protect_kind: None,
        writeback_key: Some("r:bold".to_string()),
    });
    let underline = Some(ChunkPresentation {
        bold: false,
        italic: false,
        underline: true,
        href: None,
        protect_kind: None,
        writeback_key: Some("r:u".to_string()),
    });
    let session = sample_session(vec![
        editable_chunk(0, "前文", bold.clone()),
        editable_chunk(1, "后文", underline.clone()),
    ]);
    let edits = vec![
        crate::models::EditorChunkEdit {
            index: 0,
            text: "新前文".to_string(),
        },
        crate::models::EditorChunkEdit {
            index: 1,
            text: "新后文".to_string(),
        },
    ];

    let payload =
        build_chunk_editor_writeback(&session, &edits).expect("expected docx editor payload");

    match payload {
        EditorWritebackPayload::Regions(regions) => {
            assert_eq!(regions.len(), 2);
            assert_eq!(regions[0].body, "新前文");
            assert_eq!(regions[1].body, "新后文");
            assert_eq!(regions[0].presentation, bold);
            assert_eq!(regions[1].presentation, underline);
        }
        EditorWritebackPayload::Text(_) => panic!("expected region payload"),
    }
}
