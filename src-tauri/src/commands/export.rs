use std::path::PathBuf;

use tauri::{AppHandle, State};

use crate::{
    atomic_write::write_bytes_atomically,
    documents::{normalize_text_against_source_layout, WritebackMode},
    rewrite_projection::{build_merged_regions, merged_text_from_regions},
    rewrite_writeback::execute_session_writeback,
    state::AppState,
    storage,
};

use super::support::{run_session_command, SessionCommandSource};

#[tauri::command]
pub fn export_document(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    path: String,
) -> Result<String, String> {
    let session = run_session_command(
        &app,
        &state,
        &session_id,
        SessionCommandSource::Stored,
        None,
        Ok,
    )?;
    let merged = build_merged_regions(&session, None);
    let content = normalize_text_against_source_layout(
        &session.source_text,
        &merged_text_from_regions(&merged),
    );
    let path_buf = PathBuf::from(&path);

    write_exported_text(&path_buf, &content)?;
    Ok(path)
}

#[tauri::command]
pub fn finalize_document(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
) -> Result<String, String> {
    run_session_command(
        &app,
        &state,
        &session_id,
        SessionCommandSource::Refreshed,
        Some("当前文档正在执行自动任务，请先暂停并取消后再写回原文件。"),
        |session| {
            execute_session_writeback(&session, WritebackMode::Write)?;

            // 写回成功后再清理记录，避免“写失败但记录被删”的风险。
            storage::delete_session(&app, &session_id)?;

            Ok(session.document_path)
        },
    )
}

fn write_exported_text(path: &std::path::Path, content: &str) -> Result<(), String> {
    write_bytes_atomically(path, content.as_bytes())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::test_support::{cleanup_dir, unique_test_dir};

    #[test]
    fn write_exported_text_creates_parent_dirs_and_writes_content() {
        let root = unique_test_dir("write-exported-text");
        let target = root.join("nested").join("export.txt");

        super::write_exported_text(&target, "导出内容")
            .expect("expected exported text helper to create dirs and write");

        let stored = fs::read_to_string(&target).expect("read exported text");
        assert_eq!(stored, "导出内容");
        cleanup_dir(&root);
    }
}
