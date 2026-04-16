use std::path::Path;

use super::refresh_session_from_loaded;
use crate::{
    adapters::TextRegion,
    documents::{LoadedDocumentSource, RegionSegmentationStrategy},
    models::{ChunkPreset, ChunkStatus, ChunkTask, DocumentSnapshot},
    session_refresh::test_support::{
        dirty_session_with_applied_suggestion, loaded_docx, sample_session,
    },
};

#[test]
fn refreshes_stale_plain_text_editor_capability() {
    let existing = sample_session();

    let refreshed = refresh_session_from_loaded(
        &existing,
        Path::new("/tmp/example.docx"),
        loaded_docx(),
        ChunkPreset::Paragraph,
        false,
        Some(DocumentSnapshot {
            sha256: "abc".to_string(),
        }),
    );

    assert!(refreshed.changed);
    assert!(refreshed.session.plain_text_editor_safe);
    assert_eq!(refreshed.session.plain_text_editor_block_reason, None);
    assert_eq!(
        refreshed
            .session
            .source_snapshot
            .as_ref()
            .map(|item| item.sha256.as_str()),
        Some("abc")
    );
}

#[test]
fn rebuilds_clean_session_when_chunk_preset_metadata_is_missing() {
    let now = chrono::Utc::now();
    let existing = crate::models::DocumentSession {
        id: "session-2".to_string(),
        title: "示例".to_string(),
        document_path: "/tmp/example.docx".to_string(),
        source_text: "第一句。第二句。".to_string(),
        source_snapshot: None,
        normalized_text: "第一句。第二句。".to_string(),
        write_back_supported: true,
        write_back_block_reason: None,
        plain_text_editor_safe: true,
        plain_text_editor_block_reason: None,
        chunk_preset: None,
        rewrite_headings: None,
        chunks: vec![
            ChunkTask {
                index: 0,
                source_text: "第一句。".to_string(),
                separator_after: String::new(),
                skip_rewrite: false,
                presentation: None,
                status: ChunkStatus::Idle,
                error_message: None,
            },
            ChunkTask {
                index: 1,
                source_text: "第二句。".to_string(),
                separator_after: String::new(),
                skip_rewrite: false,
                presentation: None,
                status: ChunkStatus::Idle,
                error_message: None,
            },
        ],
        suggestions: Vec::new(),
        next_suggestion_sequence: 1,
        status: crate::models::RunningState::Idle,
        created_at: now,
        updated_at: now,
    };
    let loaded = LoadedDocumentSource {
        source_text: "第一句。第二句。".to_string(),
        regions: vec![TextRegion {
            body: "第一句。第二句。".to_string(),
            skip_rewrite: false,
            presentation: None,
        }],
        region_segmentation_strategy: RegionSegmentationStrategy::PreserveBoundaries,
        write_back_supported: true,
        write_back_block_reason: None,
        plain_text_editor_safe: true,
        plain_text_editor_block_reason: None,
    };

    let refreshed = refresh_session_from_loaded(
        &existing,
        Path::new("/tmp/example.docx"),
        loaded,
        ChunkPreset::Paragraph,
        false,
        None,
    );

    assert!(refreshed.changed);
    assert_eq!(refreshed.session.chunk_preset, Some(ChunkPreset::Paragraph));
    assert_eq!(refreshed.session.rewrite_headings, Some(false));
    assert_eq!(refreshed.session.chunks.len(), 1);
    assert_eq!(refreshed.session.chunks[0].source_text, "第一句。第二句。");
}

#[test]
fn rebuilds_clean_docx_session_when_chunk_structure_is_stale() {
    let mut existing = sample_session();
    existing.source_snapshot = Some(DocumentSnapshot {
        sha256: "same".to_string(),
    });

    let refreshed = refresh_session_from_loaded(
        &existing,
        Path::new("/tmp/example.docx"),
        loaded_docx(),
        ChunkPreset::Paragraph,
        false,
        Some(DocumentSnapshot {
            sha256: "same".to_string(),
        }),
    );

    assert!(refreshed.changed);
    assert_eq!(refreshed.session.chunks.len(), 3);
    assert_eq!(refreshed.session.chunks[0].source_text, "前文");
    assert!(!refreshed.session.chunks[0].skip_rewrite);
    assert_eq!(refreshed.session.chunks[1].source_text, "E=mc^2");
    assert!(refreshed.session.chunks[1].skip_rewrite);
    assert_eq!(refreshed.session.chunks[2].source_text, "后文");
    assert!(!refreshed.session.chunks[2].skip_rewrite);
}

#[test]
fn blocks_dirty_docx_session_when_chunk_structure_is_stale() {
    let mut existing = dirty_session_with_applied_suggestion();
    existing.source_snapshot = Some(DocumentSnapshot {
        sha256: "same".to_string(),
    });

    let refreshed = refresh_session_from_loaded(
        &existing,
        Path::new("/tmp/example.docx"),
        loaded_docx(),
        ChunkPreset::Paragraph,
        false,
        Some(DocumentSnapshot {
            sha256: "same".to_string(),
        }),
    );

    assert!(refreshed.changed);
    assert_eq!(refreshed.session.suggestions.len(), 1);
    assert!(!refreshed.session.write_back_supported);
    assert!(!refreshed.session.plain_text_editor_safe);
    assert!(refreshed
        .session
        .write_back_block_reason
        .as_deref()
        .is_some_and(|reason| reason.contains("分块结构")));
}
