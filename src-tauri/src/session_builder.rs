use std::path::Path;

use chrono::{DateTime, Utc};

use crate::{
    documents::{document_format, LoadedDocumentSource, RegionSegmentationStrategy},
    models::{
        ChunkPreset, ChunkStatus, ChunkTask, DocumentSession, DocumentSnapshot, RunningState,
    },
    rewrite,
};

pub(crate) struct ChunkBuildInput<'a> {
    pub path: &'a Path,
    pub regions: Vec<crate::adapters::TextRegion>,
    pub region_segmentation_strategy: RegionSegmentationStrategy,
    pub chunk_preset: ChunkPreset,
}

pub(crate) fn build_chunks(input: ChunkBuildInput<'_>) -> Vec<ChunkTask> {
    rewrite::segment_regions_with_strategy(
        input.regions,
        input.chunk_preset,
        document_format(input.path),
        input.region_segmentation_strategy,
    )
    .into_iter()
    .enumerate()
    .map(|(index, chunk)| ChunkTask {
        index,
        source_text: chunk.text,
        separator_after: chunk.separator_after,
        skip_rewrite: chunk.skip_rewrite,
        presentation: chunk.presentation,
        status: if chunk.skip_rewrite {
            ChunkStatus::Done
        } else {
            ChunkStatus::Idle
        },
        error_message: None,
    })
    .collect()
}

pub(crate) struct CleanSessionBuildInput<'a> {
    pub session_id: String,
    pub canonical_path: &'a Path,
    pub document_path: String,
    pub loaded: LoadedDocumentSource,
    pub source_snapshot: Option<DocumentSnapshot>,
    pub chunk_preset: ChunkPreset,
    pub rewrite_headings: bool,
    pub created_at: DateTime<Utc>,
}

pub(crate) fn build_clean_session(input: CleanSessionBuildInput<'_>) -> DocumentSession {
    let LoadedDocumentSource {
        source_text,
        regions,
        region_segmentation_strategy,
        write_back_supported,
        write_back_block_reason,
        plain_text_editor_safe,
        plain_text_editor_block_reason,
    } = input.loaded;
    let normalized_text = rewrite::normalize_text(&source_text);
    let chunks = build_chunks(ChunkBuildInput {
        path: input.canonical_path,
        regions,
        region_segmentation_strategy,
        chunk_preset: input.chunk_preset,
    });
    let now = Utc::now();

    DocumentSession {
        id: input.session_id,
        title: session_title(input.canonical_path),
        document_path: input.document_path,
        source_text,
        source_snapshot: input.source_snapshot,
        normalized_text,
        write_back_supported,
        write_back_block_reason,
        plain_text_editor_safe,
        plain_text_editor_block_reason,
        chunk_preset: Some(input.chunk_preset),
        rewrite_headings: Some(input.rewrite_headings),
        chunks,
        suggestions: Vec::new(),
        next_suggestion_sequence: 1,
        status: RunningState::Idle,
        created_at: input.created_at,
        updated_at: now,
    }
}

fn session_title(path: &Path) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("未命名文稿")
        .to_string()
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::{
        adapters::TextRegion,
        documents::{LoadedDocumentSource, RegionSegmentationStrategy},
        models::{ChunkPreset, ChunkStatus, DocumentSnapshot, RunningState},
    };

    #[test]
    fn build_chunks_maps_locked_regions_to_done_status() {
        let chunks = super::build_chunks(super::ChunkBuildInput {
            path: std::path::Path::new("/tmp/example.docx"),
            regions: vec![
                TextRegion {
                    body: "前文".to_string(),
                    skip_rewrite: false,
                    presentation: None,
                },
                TextRegion {
                    body: "[公式]".to_string(),
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
            chunk_preset: ChunkPreset::Paragraph,
        });

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].status, ChunkStatus::Idle);
        assert_eq!(chunks[1].status, ChunkStatus::Done);
        assert_eq!(chunks[2].status, ChunkStatus::Idle);
    }

    #[test]
    fn build_clean_session_reuses_loaded_capabilities_and_chunk_settings() {
        let created_at = Utc::now();
        let loaded = LoadedDocumentSource {
            source_text: "前文[公式]后文".to_string(),
            regions: vec![
                TextRegion {
                    body: "前文".to_string(),
                    skip_rewrite: false,
                    presentation: None,
                },
                TextRegion {
                    body: "[公式]".to_string(),
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
            write_back_supported: false,
            write_back_block_reason: Some("blocked".to_string()),
            plain_text_editor_safe: false,
            plain_text_editor_block_reason: Some("editor blocked".to_string()),
        };

        let session = super::build_clean_session(super::CleanSessionBuildInput {
            session_id: "session-1".to_string(),
            canonical_path: std::path::Path::new("/tmp/renamed.docx"),
            document_path: "/tmp/renamed.docx".to_string(),
            loaded,
            source_snapshot: Some(DocumentSnapshot {
                sha256: "new".to_string(),
            }),
            chunk_preset: ChunkPreset::Paragraph,
            rewrite_headings: true,
            created_at,
        });

        assert_eq!(session.document_path, "/tmp/renamed.docx");
        assert_eq!(session.title, "renamed");
        assert_eq!(
            session
                .source_snapshot
                .as_ref()
                .map(|item| item.sha256.as_str()),
            Some("new")
        );
        assert_eq!(session.chunk_preset, Some(ChunkPreset::Paragraph));
        assert_eq!(session.rewrite_headings, Some(true));
        assert!(!session.write_back_supported);
        assert_eq!(session.write_back_block_reason.as_deref(), Some("blocked"));
        assert!(!session.plain_text_editor_safe);
        assert_eq!(
            session.plain_text_editor_block_reason.as_deref(),
            Some("editor blocked")
        );
        assert_eq!(session.created_at, created_at);
        assert_eq!(session.next_suggestion_sequence, 1);
        assert!(session.suggestions.is_empty());
        assert_eq!(session.status, RunningState::Idle);
    }
}
