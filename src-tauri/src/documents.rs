mod source;
#[cfg(test)]
mod test_support;
mod textual;
mod writeback;

pub(crate) use source::{
    document_session_id, is_docx_path, is_pdf_path, load_document_source, LoadedDocumentSource,
};
#[cfg(test)]
pub(crate) use test_support::writeback_slots_from_regions;
pub(crate) use textual::document_format;
#[cfg(test)]
pub(crate) use writeback::DocumentWriteback;
pub(crate) use writeback::{
    ensure_document_can_ai_rewrite, ensure_document_can_ai_rewrite_safely,
    ensure_document_can_write_back, ensure_document_source_matches_session,
    execute_document_writeback, normalize_text_against_source_layout, DocumentWritebackContext,
    OwnedDocumentWriteback, WritebackMode,
};

#[cfg(test)]
#[path = "documents_source_tests.rs"]
mod source_tests;
#[cfg(test)]
#[path = "documents_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "documents_writeback_tests.rs"]
mod writeback_tests;
