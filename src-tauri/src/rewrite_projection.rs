use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::{
    adapters::TextRegion,
    models::{ChunkPresentation, ChunkTask, DocumentSession, EditSuggestion, SuggestionDecision},
    rewrite,
};

const DOCX_BLOCK_SEPARATOR: &str = "\n\n";
pub(crate) const SUGGESTION_NOT_FOUND_ERROR: &str = "未找到对应的修改对。";

pub(crate) fn dismiss_applied_suggestions_for_chunk(
    session: &mut DocumentSession,
    index: usize,
    now: DateTime<Utc>,
) {
    for suggestion in &mut session.suggestions {
        if suggestion.chunk_index == index && suggestion.decision == SuggestionDecision::Applied {
            suggestion.decision = SuggestionDecision::Dismissed;
            suggestion.updated_at = now;
        }
    }
}

pub(crate) fn apply_preview_suggestion(
    session: &mut DocumentSession,
    index: usize,
    before_text: String,
    candidate_text: &str,
) {
    let now = Utc::now();
    dismiss_applied_suggestions_for_chunk(session, index, now);
    session.suggestions.push(EditSuggestion {
        id: "__preview__".to_string(),
        sequence: session.next_suggestion_sequence,
        chunk_index: index,
        before_text: before_text.clone(),
        after_text: candidate_text.to_string(),
        diff_spans: rewrite::build_diff(&before_text, candidate_text),
        decision: SuggestionDecision::Applied,
        created_at: now,
        updated_at: now,
    });
}

pub(crate) fn apply_suggestion_by_id(
    session: &mut DocumentSession,
    suggestion_id: &str,
    now: DateTime<Utc>,
) -> Result<usize, String> {
    let suggestion_index = find_suggestion_index(session, suggestion_id)?;
    let chunk_index = session
        .suggestions
        .get(suggestion_index)
        .ok_or_else(|| SUGGESTION_NOT_FOUND_ERROR.to_string())?
        .chunk_index;
    dismiss_applied_suggestions_for_chunk(session, chunk_index, now);
    let suggestion = session
        .suggestions
        .get_mut(suggestion_index)
        .ok_or_else(|| SUGGESTION_NOT_FOUND_ERROR.to_string())?;
    suggestion.decision = SuggestionDecision::Applied;
    suggestion.updated_at = now;
    Ok(chunk_index)
}

pub(crate) fn find_suggestion_index(
    session: &DocumentSession,
    suggestion_id: &str,
) -> Result<usize, String> {
    session
        .suggestions
        .iter()
        .position(|item| item.id == suggestion_id)
        .ok_or_else(|| SUGGESTION_NOT_FOUND_ERROR.to_string())
}

fn latest_applied_after_text<'a>(
    session: &'a DocumentSession,
    chunk: &ChunkTask,
) -> Option<&'a str> {
    session
        .suggestions
        .iter()
        .filter(|item| {
            item.chunk_index == chunk.index && item.decision == SuggestionDecision::Applied
        })
        .max_by_key(|item| item.sequence)
        .map(|item| item.after_text.as_str())
}

pub(crate) fn resolved_chunk_body(
    session: &DocumentSession,
    chunk: &ChunkTask,
    overrides: Option<&HashMap<usize, String>>,
) -> String {
    if let Some(overrides) = overrides {
        if let Some(value) = overrides.get(&chunk.index) {
            return value.clone();
        }
    }
    latest_applied_after_text(session, chunk)
        .unwrap_or(chunk.source_text.as_str())
        .to_string()
}

pub(crate) fn merged_text_from_regions(regions: &[TextRegion]) -> String {
    regions.iter().map(|region| region.body.as_str()).collect()
}

pub(crate) fn build_merged_regions(
    session: &DocumentSession,
    overrides: Option<&HashMap<usize, String>>,
) -> Vec<TextRegion> {
    let mut regions = Vec::new();
    let mut force_new_region = false;

    for chunk in &session.chunks {
        let body = resolved_chunk_body(session, chunk, overrides);
        if body.is_empty() {
            append_chunk_separator_regions(
                &mut regions,
                &chunk.separator_after,
                chunk.skip_rewrite,
                chunk.presentation.clone(),
                true,
            );
            force_new_region = chunk.separator_after.contains(DOCX_BLOCK_SEPARATOR);
            continue;
        }
        let force_chunk_boundary = force_new_region || is_whitespace_only_region_body(&body);
        append_merged_region_piece(
            &mut regions,
            &body,
            RegionAppendOptions {
                skip_rewrite: chunk.skip_rewrite,
                presentation: chunk.presentation.clone(),
                force_new_region: force_chunk_boundary,
                preserve_empty: true,
            },
        );
        append_chunk_separator_regions(
            &mut regions,
            &chunk.separator_after,
            chunk.skip_rewrite,
            chunk.presentation.clone(),
            force_chunk_boundary && body.is_empty(),
        );
        force_new_region = chunk.separator_after.contains(DOCX_BLOCK_SEPARATOR);
    }

    regions
}

fn is_whitespace_only_region_body(body: &str) -> bool {
    !body.is_empty() && body.chars().all(|ch| ch.is_whitespace())
}

pub(crate) fn chunks_preserve_docx_paragraph_boundaries(
    session: &DocumentSession,
    overrides: Option<&HashMap<usize, String>>,
) -> bool {
    session.chunks.iter().all(|chunk| {
        let body = resolved_chunk_body(session, chunk, overrides);
        !body.contains(DOCX_BLOCK_SEPARATOR)
    })
}

fn append_chunk_separator_regions(
    regions: &mut Vec<TextRegion>,
    separator_after: &str,
    skip_rewrite: bool,
    presentation: Option<ChunkPresentation>,
    force_new_region: bool,
) {
    let (current_piece, extra_empty_paragraphs) = split_separator_for_writeback(separator_after);
    append_merged_region_piece(
        regions,
        &current_piece,
        RegionAppendOptions {
            skip_rewrite,
            presentation,
            force_new_region,
            preserve_empty: false,
        },
    );
    for separator in extra_empty_paragraphs {
        append_merged_region_piece(
            regions,
            &separator,
            RegionAppendOptions {
                skip_rewrite,
                presentation: None,
                force_new_region: true,
                preserve_empty: false,
            },
        );
    }
}

fn split_separator_for_writeback(separator_after: &str) -> (String, Vec<String>) {
    let Some(first_block_index) = separator_after.find(DOCX_BLOCK_SEPARATOR) else {
        return (separator_after.to_string(), Vec::new());
    };
    let first_end = first_block_index + DOCX_BLOCK_SEPARATOR.len();
    let mut current_piece = separator_after[..first_end].to_string();
    let mut extra_empty_paragraphs = Vec::new();
    let mut remaining = &separator_after[first_end..];

    while remaining.starts_with(DOCX_BLOCK_SEPARATOR) {
        extra_empty_paragraphs.push(DOCX_BLOCK_SEPARATOR.to_string());
        remaining = &remaining[DOCX_BLOCK_SEPARATOR.len()..];
    }

    if !remaining.is_empty() {
        if let Some(last) = extra_empty_paragraphs.last_mut() {
            last.push_str(remaining);
        } else {
            current_piece.push_str(remaining);
        }
    }

    (current_piece, extra_empty_paragraphs)
}

#[derive(Clone)]
struct RegionAppendOptions {
    skip_rewrite: bool,
    presentation: Option<ChunkPresentation>,
    force_new_region: bool,
    preserve_empty: bool,
}

fn append_merged_region_piece(
    regions: &mut Vec<TextRegion>,
    text: &str,
    options: RegionAppendOptions,
) {
    if let Some(last) = matching_last_region(regions, &options) {
        last.body.push_str(text);
        return;
    }
    if text.is_empty() && !options.preserve_empty {
        return;
    }

    regions.push(TextRegion {
        body: text.to_string(),
        skip_rewrite: options.skip_rewrite,
        presentation: options.presentation,
    });
}

fn matching_last_region<'a>(
    regions: &'a mut [TextRegion],
    options: &RegionAppendOptions,
) -> Option<&'a mut TextRegion> {
    if options.force_new_region {
        return None;
    }
    let last = regions.last_mut()?;
    if last.skip_rewrite == options.skip_rewrite
        && last.presentation.as_ref() == options.presentation.as_ref()
    {
        return Some(last);
    }
    None
}
