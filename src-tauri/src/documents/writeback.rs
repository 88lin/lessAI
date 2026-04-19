use std::path::Path;

use serde::Deserialize;

use crate::{
    adapters, atomic_write::write_bytes_atomically,
    document_snapshot::ensure_document_snapshot_matches, models, rewrite,
    rewrite_unit::WritebackSlot, textual_template,
};

use super::{
    source::{is_docx_path, is_pdf_path, PDF_WRITE_BACK_UNSUPPORTED},
    textual::load_textual_template_source,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum WritebackMode {
    Validate,
    Write,
}

#[derive(Debug)]
pub(crate) enum DocumentWriteback<'a> {
    Text(&'a str),
    Slots(&'a [WritebackSlot]),
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct DocumentWritebackContext<'a> {
    pub expected_source_text: &'a str,
    pub expected_source_snapshot: Option<&'a models::DocumentSnapshot>,
    pub expected_template_signature: Option<&'a str>,
    pub expected_slot_structure_signature: Option<&'a str>,
    pub rewrite_headings: bool,
}

impl<'a> DocumentWritebackContext<'a> {
    pub(crate) fn new(
        expected_source_text: &'a str,
        expected_source_snapshot: Option<&'a models::DocumentSnapshot>,
    ) -> Self {
        Self {
            expected_source_text,
            expected_source_snapshot,
            expected_template_signature: None,
            expected_slot_structure_signature: None,
            rewrite_headings: false,
        }
    }

    pub(crate) fn from_session(session: &'a models::DocumentSession) -> Self {
        Self::new(&session.source_text, session.source_snapshot.as_ref()).with_textual_template(
            session.template_signature.as_deref(),
            session.slot_structure_signature.as_deref(),
            session.rewrite_headings.unwrap_or(false),
        )
    }

    pub(crate) fn with_textual_template(
        mut self,
        expected_template_signature: Option<&'a str>,
        expected_slot_structure_signature: Option<&'a str>,
        rewrite_headings: bool,
    ) -> Self {
        self.expected_template_signature = expected_template_signature;
        self.expected_slot_structure_signature = expected_slot_structure_signature;
        self.rewrite_headings = rewrite_headings;
        self
    }
}

#[derive(Debug)]
pub(crate) enum OwnedDocumentWriteback {
    Text(String),
    Slots(Vec<WritebackSlot>),
}

impl OwnedDocumentWriteback {
    pub(crate) fn as_document_writeback(&self) -> DocumentWriteback<'_> {
        match self {
            OwnedDocumentWriteback::Text(updated_text) => DocumentWriteback::Text(updated_text),
            OwnedDocumentWriteback::Slots(updated_slots) => DocumentWriteback::Slots(updated_slots),
        }
    }
}

pub(crate) fn ensure_document_can_write_back(path: &str) -> Result<(), String> {
    if is_pdf_path(Path::new(path)) {
        return Err(PDF_WRITE_BACK_UNSUPPORTED.to_string());
    }
    Ok(())
}

pub(crate) fn ensure_document_can_ai_rewrite(
    path: &Path,
    write_back_supported: bool,
    write_back_block_reason: Option<&str>,
) -> Result<(), String> {
    if is_pdf_path(path) {
        return Ok(());
    }
    if write_back_supported {
        return Ok(());
    }
    Err(write_back_block_reason
        .unwrap_or("当前文档暂不支持安全写回覆盖，因此不允许继续 AI 改写。")
        .to_string())
}

pub(crate) fn ensure_document_source_matches_session(
    path: &Path,
    expected_source_snapshot: Option<&models::DocumentSnapshot>,
) -> Result<(), String> {
    if is_pdf_path(path) {
        return Ok(());
    }
    load_verified_writeback_source(path, expected_source_snapshot, false).map(|_| ())
}

pub(crate) fn ensure_document_can_ai_rewrite_safely(
    path: &Path,
    expected_source_snapshot: Option<&models::DocumentSnapshot>,
    write_back_supported: bool,
    write_back_block_reason: Option<&str>,
) -> Result<(), String> {
    ensure_document_can_ai_rewrite(path, write_back_supported, write_back_block_reason)?;
    ensure_document_source_matches_session(path, expected_source_snapshot)
}

pub(crate) fn execute_document_writeback(
    path: &Path,
    context: DocumentWritebackContext<'_>,
    writeback: DocumentWriteback<'_>,
    mode: WritebackMode,
) -> Result<(), String> {
    let source = load_verified_writeback_source(
        path,
        context.expected_source_snapshot,
        context.rewrite_headings,
    )?;
    let updated = match writeback {
        DocumentWriteback::Text(updated_text) => {
            build_text_writeback_bytes(&source, context.expected_source_text, updated_text)
        }
        DocumentWriteback::Slots(updated_slots) => {
            build_slot_writeback_bytes(&source, context, updated_slots)
        }
    }?;
    finish_document_writeback(path, &updated, mode)
}

enum VerifiedWritebackSource {
    Textual(textual_template::TextTemplate),
    Docx(Vec<u8>),
}

fn load_verified_writeback_source(
    path: &Path,
    expected_source_snapshot: Option<&models::DocumentSnapshot>,
    rewrite_headings: bool,
) -> Result<VerifiedWritebackSource, String> {
    let source_bytes = ensure_document_snapshot_matches(path, expected_source_snapshot)?;
    if !is_docx_path(path) {
        let (_, template) = load_textual_template_source(path, &source_bytes, rewrite_headings)?;
        return Ok(VerifiedWritebackSource::Textual(template));
    }

    Ok(VerifiedWritebackSource::Docx(source_bytes))
}

fn build_text_writeback_bytes(
    source: &VerifiedWritebackSource,
    expected_source_text: &str,
    updated_text: &str,
) -> Result<Vec<u8>, String> {
    match source {
        VerifiedWritebackSource::Textual(_) => Ok(normalize_text_against_source_layout(
            expected_source_text,
            updated_text,
        )
        .into_bytes()),
        VerifiedWritebackSource::Docx(current_bytes) => {
            adapters::docx::DocxAdapter::write_updated_text(
                current_bytes,
                expected_source_text,
                updated_text,
            )
        }
    }
}

pub(crate) fn normalize_text_against_source_layout(
    expected_source_text: &str,
    updated_text: &str,
) -> String {
    let line_ending = rewrite::detect_line_ending(expected_source_text);
    let mut normalized = updated_text.to_string();
    if !rewrite::has_trailing_spaces_per_line(expected_source_text) {
        normalized = rewrite::strip_trailing_spaces_per_line(&normalized);
    }
    rewrite::convert_line_endings(&normalized, line_ending)
}

fn build_slot_writeback_bytes(
    source: &VerifiedWritebackSource,
    context: DocumentWritebackContext<'_>,
    updated_slots: &[WritebackSlot],
) -> Result<Vec<u8>, String> {
    match source {
        VerifiedWritebackSource::Textual(template) => {
            textual_template::validate::ensure_template_signature(
                context.expected_template_signature,
                template,
            )?;
            textual_template::validate::ensure_slot_structure_signature(
                context.expected_slot_structure_signature,
                updated_slots,
            )?;
            let rebuilt = textual_template::rebuild::rebuild_text(template, updated_slots)?;
            Ok(
                normalize_text_against_source_layout(context.expected_source_text, &rebuilt)
                    .into_bytes(),
            )
        }
        VerifiedWritebackSource::Docx(current_bytes) => {
            adapters::docx::DocxAdapter::write_updated_slots(
                current_bytes,
                context.expected_source_text,
                updated_slots,
            )
        }
    }
}

fn finish_document_writeback(
    path: &Path,
    updated: &[u8],
    mode: WritebackMode,
) -> Result<(), String> {
    match mode {
        WritebackMode::Validate => Ok(()),
        WritebackMode::Write => write_bytes_atomically(path, updated),
    }
}

#[cfg(test)]
#[path = "writeback_internal_tests.rs"]
mod internal_tests;
