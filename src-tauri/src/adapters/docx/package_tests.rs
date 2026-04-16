use super::package::load_docx_parts;
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
