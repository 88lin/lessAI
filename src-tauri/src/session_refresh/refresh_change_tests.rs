use std::path::Path;

use chrono::Utc;

use super::refresh_session_from_loaded;
use crate::{
    adapters::TextRegion,
    documents::{LoadedDocumentSource, RegionSegmentationStrategy},
    models::{ChunkPreset, DocumentSnapshot, EditSuggestion, RunningState, SuggestionDecision},
    session_refresh::test_support::{
        dirty_session_with_applied_suggestion, loaded_docx, sample_session,
    },
};

#[test]
fn rebuilds_clean_session_when_snapshot_changes_even_if_text_is_same() {
    let mut existing = sample_session();
    existing.source_snapshot = Some(DocumentSnapshot {
        sha256: "old".to_string(),
    });

    let refreshed = refresh_session_from_loaded(
        &existing,
        Path::new("/tmp/example.docx"),
        loaded_docx(),
        ChunkPreset::Paragraph,
        false,
        Some(DocumentSnapshot {
            sha256: "new".to_string(),
        }),
    );

    assert!(refreshed.changed);
    assert_eq!(
        refreshed
            .session
            .source_snapshot
            .as_ref()
            .map(|item| item.sha256.as_str()),
        Some("new")
    );
    assert_eq!(refreshed.session.chunks.len(), 3);
    assert!(refreshed.session.write_back_supported);
    assert_eq!(refreshed.session.write_back_block_reason, None);
}

#[test]
fn blocks_dirty_session_when_snapshot_changes_even_if_text_is_same() {
    let existing = dirty_session_with_applied_suggestion();

    let refreshed = refresh_session_from_loaded(
        &existing,
        Path::new("/tmp/example.docx"),
        loaded_docx(),
        ChunkPreset::Paragraph,
        false,
        Some(DocumentSnapshot {
            sha256: "new".to_string(),
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
        .is_some_and(|reason| reason.contains("外部发生变化")));
    assert_eq!(
        refreshed
            .session
            .source_snapshot
            .as_ref()
            .map(|item| item.sha256.as_str()),
        Some("old")
    );
}

#[test]
fn rebuilds_snapshotless_clean_session_when_source_changes() {
    let existing = sample_session();
    let loaded = LoadedDocumentSource {
        source_text: "新前文E=mc^2新后文".to_string(),
        regions: vec![
            TextRegion {
                body: "新前文".to_string(),
                skip_rewrite: false,
                presentation: None,
            },
            TextRegion {
                body: "E=mc^2".to_string(),
                skip_rewrite: true,
                presentation: None,
            },
            TextRegion {
                body: "新后文".to_string(),
                skip_rewrite: false,
                presentation: None,
            },
        ],
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
        Some(DocumentSnapshot {
            sha256: "new".to_string(),
        }),
    );

    assert!(refreshed.changed);
    assert_eq!(refreshed.session.source_text, "新前文E=mc^2新后文");
    assert_eq!(
        refreshed
            .session
            .source_snapshot
            .as_ref()
            .map(|item| item.sha256.as_str()),
        Some("new")
    );
    assert!(refreshed.session.suggestions.is_empty());
}

#[test]
fn blocks_snapshotless_dirty_session_when_source_changes() {
    let mut existing = sample_session();
    existing.suggestions.push(EditSuggestion {
        id: "suggestion-1".to_string(),
        sequence: 1,
        chunk_index: 0,
        before_text: existing.source_text.clone(),
        after_text: "改写后正文".to_string(),
        diff_spans: Vec::new(),
        decision: SuggestionDecision::Applied,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    });
    existing.status = RunningState::Completed;

    let loaded = LoadedDocumentSource {
        source_text: "新前文E=mc^2新后文".to_string(),
        regions: vec![TextRegion {
            body: "新前文E=mc^2新后文".to_string(),
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
        Some(DocumentSnapshot {
            sha256: "new".to_string(),
        }),
    );

    assert!(refreshed.changed);
    assert_eq!(refreshed.session.suggestions.len(), 1);
    assert!(!refreshed.session.write_back_supported);
    assert!(!refreshed.session.plain_text_editor_safe);
    assert_eq!(refreshed.session.source_snapshot, None);
}

#[test]
fn blocks_snapshotless_dirty_session_even_when_source_text_is_unchanged() {
    let mut existing = sample_session();
    existing.suggestions.push(EditSuggestion {
        id: "suggestion-1".to_string(),
        sequence: 1,
        chunk_index: 0,
        before_text: existing.source_text.clone(),
        after_text: "改写后正文".to_string(),
        diff_spans: Vec::new(),
        decision: SuggestionDecision::Applied,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    });
    existing.status = RunningState::Completed;

    let refreshed = refresh_session_from_loaded(
        &existing,
        Path::new("/tmp/example.docx"),
        loaded_docx(),
        ChunkPreset::Paragraph,
        false,
        Some(DocumentSnapshot {
            sha256: "new".to_string(),
        }),
    );

    assert!(refreshed.changed);
    assert_eq!(refreshed.session.suggestions.len(), 1);
    assert!(!refreshed.session.write_back_supported);
    assert!(!refreshed.session.plain_text_editor_safe);
    assert_eq!(refreshed.session.source_snapshot, None);
}
