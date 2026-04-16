use chrono::Utc;

use crate::{
    adapters::TextRegion,
    documents::{LoadedDocumentSource, RegionSegmentationStrategy},
    models::{
        ChunkPreset, ChunkStatus, ChunkTask, DocumentSession, DocumentSnapshot, EditSuggestion,
        RunningState, SuggestionDecision,
    },
};

pub(super) fn sample_session() -> DocumentSession {
    let now = Utc::now();
    DocumentSession {
        id: "session-1".to_string(),
        title: "示例".to_string(),
        document_path: "/tmp/example.docx".to_string(),
        source_text: "前文E=mc^2后文".to_string(),
        source_snapshot: None,
        normalized_text: "前文E=mc^2后文".to_string(),
        write_back_supported: true,
        write_back_block_reason: None,
        plain_text_editor_safe: false,
        plain_text_editor_block_reason: Some(
            "当前文档包含行内锁定内容（如公式、分页符或占位符），暂不支持在纯文本编辑器中直接写回。"
                .to_string(),
        ),
        chunk_preset: Some(ChunkPreset::Paragraph),
        rewrite_headings: Some(false),
        chunks: vec![ChunkTask {
            index: 0,
            source_text: "前文E=mc^2后文".to_string(),
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

pub(super) fn dirty_session_with_applied_suggestion() -> DocumentSession {
    let mut session = sample_session();
    let now = Utc::now();
    session.source_snapshot = Some(DocumentSnapshot {
        sha256: "old".to_string(),
    });
    session.suggestions.push(EditSuggestion {
        id: "suggestion-1".to_string(),
        sequence: 1,
        chunk_index: 0,
        before_text: "前文E=mc^2后文".to_string(),
        after_text: "改写后正文".to_string(),
        diff_spans: Vec::new(),
        decision: SuggestionDecision::Applied,
        created_at: now,
        updated_at: now,
    });
    session.status = RunningState::Completed;
    session
}

pub(super) fn loaded_docx() -> LoadedDocumentSource {
    LoadedDocumentSource {
        source_text: "前文E=mc^2后文".to_string(),
        regions: vec![
            TextRegion {
                body: "前文".to_string(),
                skip_rewrite: false,
                presentation: None,
            },
            TextRegion {
                body: "E=mc^2".to_string(),
                skip_rewrite: true,
                presentation: None,
            },
            TextRegion {
                body: "后文".to_string(),
                skip_rewrite: false,
                presentation: None,
            },
        ],
        region_segmentation_strategy: RegionSegmentationStrategy::PreserveBoundaries,
        write_back_supported: true,
        write_back_block_reason: None,
        plain_text_editor_safe: true,
        plain_text_editor_block_reason: None,
    }
}
