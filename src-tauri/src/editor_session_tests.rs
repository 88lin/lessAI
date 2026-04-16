use std::cell::Cell;

use std::{
    fs,
    path::{Path, PathBuf},
};

use super::{
    ensure_editor_base_snapshot_matches_path, DocumentSession, EDITOR_BASE_SNAPSHOT_EXPIRED_ERROR,
    EDITOR_BASE_SNAPSHOT_MISSING_ERROR,
};
use crate::{
    document_snapshot::capture_document_snapshot,
    editor_writeback::EditorWritebackPayload,
    test_support::{cleanup_dir, sample_clean_session, unique_test_dir},
};

fn sample_session(path: &PathBuf) -> crate::models::DocumentSession {
    sample_clean_session("session-1", &path.to_string_lossy(), "正文")
}

fn sample_finished_session() -> DocumentSession {
    sample_session(&PathBuf::from("/tmp/example.txt"))
}

#[test]
fn rejects_missing_editor_base_snapshot() {
    let root = unique_test_dir("missing");
    fs::create_dir_all(&root).expect("create root");
    let target = root.join("sample.txt");
    fs::write(&target, "正文").expect("write source");

    let error = ensure_editor_base_snapshot_matches_path(&target, None)
        .expect_err("expected missing editor snapshot to be rejected");

    assert_eq!(error, EDITOR_BASE_SNAPSHOT_MISSING_ERROR);
    cleanup_dir(&root);
}

#[test]
fn rejects_stale_editor_base_snapshot() {
    let root = unique_test_dir("stale");
    fs::create_dir_all(&root).expect("create root");
    let target = root.join("sample.txt");
    fs::write(&target, "旧正文").expect("write source");
    let original_snapshot = capture_document_snapshot(&target).expect("capture original");
    fs::write(&target, "新正文").expect("simulate external change");

    let error = ensure_editor_base_snapshot_matches_path(&target, Some(&original_snapshot))
        .expect_err("expected stale editor snapshot to be rejected");

    assert_eq!(error, EDITOR_BASE_SNAPSHOT_EXPIRED_ERROR);
    cleanup_dir(&root);
}

#[test]
fn path_guard_uses_editor_base_snapshot_instead_of_session_snapshot() {
    let root = unique_test_dir("path-guard");
    fs::create_dir_all(&root).expect("create root");
    let target = root.join("sample.txt");
    fs::write(&target, "旧正文").expect("write original");
    let original_snapshot = capture_document_snapshot(&target).expect("capture original");
    fs::write(&target, "新正文").expect("simulate external change");

    let mut session = sample_session(&target);
    session.source_snapshot = Some(capture_document_snapshot(&target).expect("capture current"));

    let error = ensure_editor_base_snapshot_matches_path(
        Path::new(&session.document_path),
        Some(&original_snapshot),
    )
    .expect_err("expected wrapper to reject stale editor snapshot");

    assert_eq!(error, EDITOR_BASE_SNAPSHOT_EXPIRED_ERROR);
    cleanup_dir(&root);
}

#[test]
fn run_loaded_editor_writeback_stops_before_execute_when_build_fails() {
    let execute_calls = Cell::new(0);

    let error = match super::run_loaded_editor_writeback(
        sample_finished_session(),
        crate::documents::WritebackMode::Validate,
        |_| Err::<EditorWritebackPayload, String>("build failed".to_string()),
        |_, _, _| {
            execute_calls.set(execute_calls.get() + 1);
            Ok(())
        },
        |_| Ok(()),
    ) {
        Ok(_) => panic!("expected build failure to short-circuit execute path"),
        Err(error) => error,
    };

    assert_eq!(error, "build failed");
    assert_eq!(execute_calls.get(), 0);
}

#[test]
fn run_loaded_editor_writeback_supports_validate_mode() {
    let finish_calls = Cell::new(0);

    super::run_loaded_editor_writeback(
        sample_finished_session(),
        crate::documents::WritebackMode::Validate,
        |_| Ok(EditorWritebackPayload::Text("改写正文".to_string())),
        |session, payload, mode| {
            assert_eq!(session.id, "session-1");
            assert_eq!(mode, crate::documents::WritebackMode::Validate);
            match payload {
                EditorWritebackPayload::Text(text) => {
                    assert_eq!(text, "改写正文");
                    Ok(())
                }
                EditorWritebackPayload::Regions(_) => Err("expected text payload".to_string()),
            }
        },
        |_| {
            finish_calls.set(finish_calls.get() + 1);
            Ok(())
        },
    )
    .expect("expected validate path to execute unified flow");

    assert_eq!(finish_calls.get(), 1);
}

#[test]
fn run_loaded_editor_writeback_supports_write_mode() {
    let finish_calls = Cell::new(0);

    let result = super::run_loaded_editor_writeback(
        sample_finished_session(),
        crate::documents::WritebackMode::Write,
        |_| Ok(EditorWritebackPayload::Text("改写正文".to_string())),
        |session, payload, mode| {
            assert_eq!(session.id, "session-1");
            assert_eq!(mode, crate::documents::WritebackMode::Write);
            match payload {
                EditorWritebackPayload::Text(text) => {
                    assert_eq!(text, "改写正文");
                    Ok(())
                }
                EditorWritebackPayload::Regions(_) => Err("expected text payload".to_string()),
            }
        },
        |session| {
            finish_calls.set(finish_calls.get() + 1);
            let mut rebuilt = session;
            rebuilt.title = "重建后".to_string();
            Ok(rebuilt)
        },
    )
    .expect("expected write path to execute unified flow");

    assert_eq!(result.title, "重建后");
    assert_eq!(finish_calls.get(), 1);
}
