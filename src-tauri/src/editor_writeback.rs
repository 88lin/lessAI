use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use crate::{
    adapters::TextRegion,
    documents::{
        ensure_document_can_write_back, execute_document_writeback, is_docx_path,
        normalize_text_against_source_layout, OwnedDocumentWriteback, WritebackMode,
    },
    models::{ChunkStatus, DocumentSession, EditorChunkEdit, RunningState},
    rewrite_projection::build_merged_regions,
};

const EDITOR_WRITEBACK_CONFLICT_ERROR: &str =
    "该文档存在修订记录或进度，为避免冲突，请先“覆写并清理记录”或“重置记录”后再编辑。";

pub(crate) type EditorWritebackPayload = OwnedDocumentWriteback;

pub(crate) fn ensure_session_can_use_plain_text_editor(
    session: &DocumentSession,
) -> Result<(), String> {
    ensure_document_can_write_back(&session.document_path)?;
    if !session.plain_text_editor_safe {
        return Err(session
            .plain_text_editor_block_reason
            .clone()
            .unwrap_or_else(|| "当前文档暂不支持进入编辑模式。".to_string()));
    }
    if !plain_text_editor_session_is_clean(session) {
        return Err(EDITOR_WRITEBACK_CONFLICT_ERROR.to_string());
    }
    Ok(())
}

fn normalize_editor_writeback_content(
    document_path: &str,
    source_text: &str,
    content: &str,
) -> String {
    if is_docx_path(Path::new(document_path)) {
        return content.to_string();
    }
    normalize_text_against_source_layout(source_text, content)
}

pub(crate) fn build_plain_text_editor_writeback(
    session: &DocumentSession,
    content: &str,
) -> Result<EditorWritebackPayload, String> {
    if content.trim().is_empty() {
        return Err("文档内容为空，无法保存。".to_string());
    }
    ensure_session_can_use_plain_text_editor(session)?;
    if is_docx_path(Path::new(&session.document_path)) {
        return Err("docx 编辑模式必须按片段保存，不能再走整篇纯文本写回。".to_string());
    }

    Ok(EditorWritebackPayload::Text(
        normalize_editor_writeback_content(&session.document_path, &session.source_text, content),
    ))
}

pub(crate) fn build_chunk_editor_writeback(
    session: &DocumentSession,
    edits: &[EditorChunkEdit],
) -> Result<EditorWritebackPayload, String> {
    Ok(EditorWritebackPayload::Regions(
        build_updated_regions_from_chunk_edits(session, edits)?,
    ))
}

pub(crate) fn execute_editor_writeback(
    session: &DocumentSession,
    payload: &EditorWritebackPayload,
    mode: WritebackMode,
) -> Result<(), String> {
    execute_document_writeback(
        Path::new(&session.document_path),
        &session.source_text,
        session.source_snapshot.as_ref(),
        payload.as_document_writeback(),
        mode,
    )
}

fn build_updated_regions_from_chunk_edits(
    session: &DocumentSession,
    edits: &[EditorChunkEdit],
) -> Result<Vec<TextRegion>, String> {
    with_chunk_edit_overrides(session, edits, |session, overrides| {
        if !is_docx_path(Path::new(&session.document_path)) {
            return Err("当前仅 docx 支持按片段编辑写回。".to_string());
        }
        Ok(build_merged_regions(session, Some(overrides)))
    })
}

fn plain_text_editor_session_is_clean(session: &DocumentSession) -> bool {
    session.status == RunningState::Idle
        && session.suggestions.is_empty()
        && session
            .chunks
            .iter()
            .all(|chunk| chunk.status == ChunkStatus::Idle || chunk.skip_rewrite)
}

fn collect_chunk_edit_overrides(
    session: &DocumentSession,
    edits: &[EditorChunkEdit],
) -> Result<HashMap<usize, String>, String> {
    let editable_indices = session
        .chunks
        .iter()
        .filter(|chunk| !chunk.skip_rewrite)
        .map(|chunk| chunk.index)
        .collect::<HashSet<_>>();
    if edits.len() != editable_indices.len() {
        return Err("编辑器提交的可编辑片段数量与当前会话不一致，请重新进入编辑模式。".to_string());
    }

    let mut overrides = HashMap::with_capacity(edits.len());
    for edit in edits {
        if !editable_indices.contains(&edit.index) {
            return Err(format!(
                "编辑器提交了不可编辑或不存在的片段 #{}, 无法安全写回。",
                edit.index + 1
            ));
        }
        if overrides.insert(edit.index, edit.text.clone()).is_some() {
            return Err(format!(
                "编辑器提交了重复的片段 #{}, 无法安全写回。",
                edit.index + 1
            ));
        }
    }

    Ok(overrides)
}

fn with_chunk_edit_overrides<T, Apply>(
    session: &DocumentSession,
    edits: &[EditorChunkEdit],
    apply: Apply,
) -> Result<T, String>
where
    Apply: FnOnce(&DocumentSession, &HashMap<usize, String>) -> Result<T, String>,
{
    ensure_session_can_use_plain_text_editor(session)?;
    let overrides = collect_chunk_edit_overrides(session, edits)?;
    apply(session, &overrides)
}

#[cfg(test)]
#[path = "editor_writeback_tests.rs"]
mod tests;
