use std::fs;

use crate::adapters::TextRegion;
use crate::test_support::{cleanup_dir, write_temp_file};

#[test]
fn finish_document_writeback_skips_disk_write_in_validate_mode() {
    let (root, target) = write_temp_file("document-writeback-validate", "txt", b"original");

    super::finish_document_writeback(&target, b"updated", super::WritebackMode::Validate)
        .expect("expected validate mode to skip disk write");

    let stored = fs::read(&target).expect("read untouched file");
    assert_eq!(stored, b"original");
    cleanup_dir(&root);
}

#[test]
fn finish_document_writeback_persists_bytes_in_write_mode() {
    let (root, target) = write_temp_file("document-writeback-write", "txt", b"original");

    super::finish_document_writeback(&target, b"updated", super::WritebackMode::Write)
        .expect("expected write mode to persist bytes");

    let stored = fs::read(&target).expect("read updated file");
    assert_eq!(stored, b"updated");
    cleanup_dir(&root);
}

#[test]
fn build_text_writeback_bytes_returns_plain_text_bytes_for_plain_text_source() {
    let bytes = super::build_text_writeback_bytes(
        &super::VerifiedWritebackSource::PlainText,
        "原始内容",
        "新的内容",
    )
    .expect("expected plain text writeback bytes");

    assert_eq!(bytes, "新的内容".as_bytes());
}

#[test]
fn build_region_writeback_bytes_rejects_plain_text_source() {
    let error = super::build_region_writeback_bytes(
        &super::VerifiedWritebackSource::PlainText,
        "原始内容",
        &[TextRegion {
            body: "新的内容".to_string(),
            skip_rewrite: false,
            presentation: None,
        }],
    )
    .expect_err("expected plain text region writeback to be rejected");

    assert_eq!(error, "当前仅 docx 支持按片段写回。");
}
