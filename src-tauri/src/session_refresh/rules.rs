use std::path::Path;

use crate::{
    documents::{document_format, RegionSegmentationStrategy},
    models::{
        ChunkPreset, ChunkStatus, ChunkTask, DocumentSession, DocumentSnapshot, RunningState,
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ChunkRefreshAction {
    Keep,
    Rebuild,
    Block,
}

pub(super) fn source_snapshot_changed(
    existing: &DocumentSession,
    current_snapshot: Option<&DocumentSnapshot>,
) -> bool {
    existing.source_snapshot.as_ref() != current_snapshot
}

pub(super) fn session_can_rebuild_cleanly(session: &DocumentSession) -> bool {
    session.status == RunningState::Idle
        && session.suggestions.is_empty()
        && session.chunks.iter().all(|chunk| {
            (chunk.skip_rewrite && chunk.status == ChunkStatus::Done)
                || (!chunk.skip_rewrite && chunk.status == ChunkStatus::Idle)
        })
}

pub(super) fn decide_chunk_refresh(
    session: &DocumentSession,
    expected_chunks: &[ChunkTask],
    strategy: RegionSegmentationStrategy,
    chunk_preset: ChunkPreset,
    rewrite_headings: bool,
) -> ChunkRefreshAction {
    if has_stale_preserved_chunk_structure(
        session,
        expected_chunks,
        strategy,
        chunk_preset,
        rewrite_headings,
    ) {
        return if session_can_rebuild_cleanly(session) {
            ChunkRefreshAction::Rebuild
        } else {
            ChunkRefreshAction::Block
        };
    }

    if should_rebuild_chunks(session, expected_chunks, chunk_preset, rewrite_headings) {
        return ChunkRefreshAction::Rebuild;
    }

    ChunkRefreshAction::Keep
}

fn should_rebuild_chunks(
    session: &DocumentSession,
    expected_chunks: &[ChunkTask],
    chunk_preset: ChunkPreset,
    rewrite_headings: bool,
) -> bool {
    if !session.suggestions.is_empty() {
        return false;
    }

    let settings_changed = session.chunk_preset != Some(chunk_preset)
        || session.rewrite_headings != Some(rewrite_headings);
    if settings_changed {
        return true;
    }

    let rebuilt = expected_chunks
        .iter()
        .map(|chunk| format!("{}{}", chunk.source_text, chunk.separator_after))
        .collect::<String>();
    let format = document_format(Path::new(&session.document_path));
    let has_inline_newlines = expected_chunks.iter().any(chunk_has_inline_newlines);
    let allow_inline_newlines = format == crate::models::DocumentFormat::Tex;

    rebuilt != session.source_text
        || (!matches!(chunk_preset, ChunkPreset::Paragraph)
            && has_inline_newlines
            && !allow_inline_newlines)
}

fn has_stale_preserved_chunk_structure(
    session: &DocumentSession,
    expected_chunks: &[ChunkTask],
    strategy: RegionSegmentationStrategy,
    chunk_preset: ChunkPreset,
    rewrite_headings: bool,
) -> bool {
    strategy == RegionSegmentationStrategy::PreserveBoundaries
        && session.chunk_preset == Some(chunk_preset)
        && session.rewrite_headings == Some(rewrite_headings)
        && !chunk_structures_match(&session.chunks, expected_chunks)
}

fn chunk_structures_match(current: &[ChunkTask], expected: &[ChunkTask]) -> bool {
    current.len() == expected.len()
        && current.iter().zip(expected.iter()).all(|(left, right)| {
            left.source_text == right.source_text
                && left.separator_after == right.separator_after
                && left.skip_rewrite == right.skip_rewrite
                && left.presentation == right.presentation
        })
}

fn chunk_has_inline_newlines(chunk: &ChunkTask) -> bool {
    !chunk.skip_rewrite && (chunk.source_text.contains('\n') || chunk.source_text.contains('\r'))
}
