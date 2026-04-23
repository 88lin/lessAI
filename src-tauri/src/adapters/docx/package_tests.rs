use super::package::{format_docx_zip_error, load_docx_parts};
use crate::test_support::build_docx_entries;

#[test]
fn load_docx_parts_reads_document_xml_and_defaults_missing_optional_parts() {
    let bytes = build_docx_entries(&[(
        "word/document.xml",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body><w:p><w:r><w:t>正文</w:t></w:r></w:p></w:body>
</w:document>"#,
    )]);

    let parts = load_docx_parts(&bytes).expect("load docx parts");

    assert!(parts.document_xml.contains("<w:document"));
    assert!(parts.relationships_xml.is_none());
    assert!(parts.numbering_xml.is_none());
    assert!(parts.styles_xml.is_none());
}

#[test]
fn load_docx_parts_rejects_missing_document_xml() {
    let bytes = build_docx_entries(&[(
        "word/styles.xml",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"/>"#,
    )]);

    let error = load_docx_parts(&bytes).expect_err("expected missing document.xml to fail");

    assert!(error.contains("word/document.xml"));
}

#[test]
fn load_docx_parts_reports_eocd_hint_for_truncated_or_non_zip_docx() {
    let bogus = b"not-a-zip-docx";

    let error = load_docx_parts(bogus).expect_err("expected invalid zip docx to fail");

    assert!(
        error.contains("EOCD") || error.contains("并非真实 .docx"),
        "unexpected error: {error}"
    );
}

#[test]
fn format_docx_zip_error_reports_eocd_hint() {
    let detail = "invalid Zip archive: Could not find EOCD";
    let error =
        format_docx_zip_error(zip::result::ZipError::InvalidArchive(detail.to_string().into()));

    assert!(error.contains("EOCD"), "unexpected error: {error}");
}
