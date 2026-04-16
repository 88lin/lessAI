use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
};

use chrono::Utc;
use uuid::Uuid;
use zip::{write::FileOptions, ZipWriter};

use crate::models::{ChunkPreset, DocumentSession, RunningState};

pub(crate) fn unique_test_dir(name: &str) -> PathBuf {
    env::temp_dir().join(format!("lessai-{name}-{}", Uuid::new_v4()))
}

pub(crate) fn cleanup_dir(path: &Path) {
    let _ = fs::remove_dir_all(path);
}

pub(crate) fn write_temp_file(name: &str, ext: &str, contents: &[u8]) -> (PathBuf, PathBuf) {
    let root = unique_test_dir(name);
    fs::create_dir_all(&root).expect("create root");
    let target = root.join(format!("sample.{ext}"));
    fs::write(&target, contents).expect("write temp file");
    (root, target)
}

pub(crate) fn build_docx_entries(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut out = Vec::new();
    let cursor = std::io::Cursor::new(&mut out);
    let mut zip = ZipWriter::new(cursor);
    let options = FileOptions::<()>::default();

    for (name, contents) in entries {
        zip.start_file(*name, options).expect("start zip entry");
        zip.write_all(contents.as_bytes()).expect("write zip entry");
    }

    zip.finish().expect("finish docx");
    out
}

pub(crate) fn build_minimal_docx(document_xml: &str) -> Vec<u8> {
    build_docx_entries(&[("word/document.xml", document_xml)])
}

pub(crate) fn sample_clean_session(
    id: &str,
    document_path: &str,
    source_text: &str,
) -> DocumentSession {
    let now = Utc::now();
    DocumentSession {
        id: id.to_string(),
        title: "示例".to_string(),
        document_path: document_path.to_string(),
        source_text: source_text.to_string(),
        source_snapshot: None,
        normalized_text: source_text.to_string(),
        write_back_supported: true,
        write_back_block_reason: None,
        plain_text_editor_safe: true,
        plain_text_editor_block_reason: None,
        chunk_preset: Some(ChunkPreset::Paragraph),
        rewrite_headings: Some(false),
        chunks: Vec::new(),
        suggestions: Vec::new(),
        next_suggestion_sequence: 1,
        status: RunningState::Idle,
        created_at: now,
        updated_at: now,
    }
}
