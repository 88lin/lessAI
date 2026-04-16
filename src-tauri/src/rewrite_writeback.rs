use std::{collections::HashMap, path::Path};

use log::{error, info};

use crate::{
    documents::{
        ensure_document_can_ai_rewrite, ensure_document_can_write_back, execute_document_writeback,
        is_docx_path, OwnedDocumentWriteback, WritebackMode,
    },
    models::{DocumentSession, SuggestionDecision},
    observability::{document_kind_label, writeback_mode_label},
    rewrite_permissions::ensure_chunk_can_rewrite,
    rewrite_permissions::CHUNK_INDEX_OUT_OF_RANGE_ERROR,
    rewrite_projection::{
        apply_preview_suggestion, build_merged_regions, chunks_preserve_docx_paragraph_boundaries,
        merged_text_from_regions,
    },
};

type SessionWritebackPlan = OwnedDocumentWriteback;

pub(crate) fn validate_candidate_batch_writeback(
    session: &DocumentSession,
    overrides: &HashMap<usize, String>,
) -> Result<(), String> {
    let preview = build_preview_session(session, overrides)?;
    execute_session_writeback(&preview, WritebackMode::Validate)
}

pub(crate) fn execute_session_writeback(
    session: &DocumentSession,
    mode: WritebackMode,
) -> Result<(), String> {
    let path = Path::new(&session.document_path);
    info!(
        "session writeback started: session_id={} mode={} document_kind={} path={}",
        session.id,
        writeback_mode_label(mode),
        document_kind_label(&session.document_path),
        session.document_path,
    );

    let result = (|| {
        if mode == WritebackMode::Write {
            ensure_document_can_write_back(&session.document_path)?;
        }
        ensure_applied_suggestions_target_rewriteable(session)?;
        ensure_document_can_ai_rewrite(
            path,
            session.write_back_supported,
            session.write_back_block_reason.as_deref(),
        )?;

        let plan = build_session_writeback_plan(session);
        execute_document_writeback(
            path,
            &session.source_text,
            session.source_snapshot.as_ref(),
            plan.as_document_writeback(),
            mode,
        )
    })();

    match &result {
        Ok(()) => info!(
            "session writeback finished: session_id={} mode={} document_kind={} path={}",
            session.id,
            writeback_mode_label(mode),
            document_kind_label(&session.document_path),
            session.document_path,
        ),
        Err(message) => error!(
            "session writeback failed: session_id={} mode={} document_kind={} path={} error={message}",
            session.id,
            writeback_mode_label(mode),
            document_kind_label(&session.document_path),
            session.document_path,
        ),
    }

    result
}

fn build_preview_session(
    session: &DocumentSession,
    overrides: &HashMap<usize, String>,
) -> Result<DocumentSession, String> {
    ensure_override_targets_rewriteable(session, overrides.keys().copied())?;
    let mut preview = session.clone();
    for (index, candidate_text) in overrides {
        let before_text = preview
            .chunks
            .get(*index)
            .ok_or_else(|| CHUNK_INDEX_OUT_OF_RANGE_ERROR.to_string())?
            .source_text
            .clone();
        apply_preview_suggestion(&mut preview, *index, before_text, candidate_text);
    }
    Ok(preview)
}

fn build_session_writeback_plan(session: &DocumentSession) -> SessionWritebackPlan {
    if !is_docx_path(Path::new(&session.document_path)) {
        return SessionWritebackPlan::Text(merged_text_from_regions(&build_merged_regions(
            session, None,
        )));
    }

    let updated_regions = build_merged_regions(session, None);
    if chunks_preserve_docx_paragraph_boundaries(session, None) {
        return SessionWritebackPlan::Regions(updated_regions);
    }

    SessionWritebackPlan::Text(merged_text_from_regions(&updated_regions))
}

fn ensure_override_targets_rewriteable(
    session: &DocumentSession,
    indices: impl IntoIterator<Item = usize>,
) -> Result<(), String> {
    for index in indices {
        ensure_chunk_can_rewrite(session, index)?;
    }
    Ok(())
}

fn ensure_applied_suggestions_target_rewriteable(session: &DocumentSession) -> Result<(), String> {
    for suggestion in session
        .suggestions
        .iter()
        .filter(|item| item.decision == SuggestionDecision::Applied)
    {
        ensure_chunk_can_rewrite(session, suggestion.chunk_index)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "rewrite_writeback_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "rewrite_writeback_fixture_tests.rs"]
mod fixture_tests;
