use std::path::Path;

use crate::{documents::ensure_document_can_ai_rewrite_safely, models::DocumentSession};

pub(crate) const CHUNK_INDEX_OUT_OF_RANGE_ERROR: &str = "片段索引越界。";

pub(crate) fn protected_chunk_rewrite_error(index: usize) -> String {
    format!("第 {} 段属于保护区，不允许 AI 改写。", index + 1)
}

pub(crate) fn ensure_session_can_rewrite(session: &DocumentSession) -> Result<(), String> {
    ensure_document_can_ai_rewrite_safely(
        Path::new(&session.document_path),
        session.source_snapshot.as_ref(),
        session.write_back_supported,
        session.write_back_block_reason.as_deref(),
    )
}

pub(crate) fn ensure_chunk_can_rewrite(
    session: &DocumentSession,
    index: usize,
) -> Result<(), String> {
    let chunk = session
        .chunks
        .get(index)
        .ok_or_else(|| CHUNK_INDEX_OUT_OF_RANGE_ERROR.to_string())?;
    if chunk.skip_rewrite {
        return Err(protected_chunk_rewrite_error(index));
    }
    Ok(())
}
