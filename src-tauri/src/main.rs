#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod models;
mod rewrite;
mod storage;

use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use chrono::Utc;
use models::{
    AppSettings, ChunkCompletedEvent, ChunkStatus, ChunkTask, DocumentSession, EditSuggestion,
    RewriteFailedEvent, RewriteMode, RewriteProgress, RunningState, SessionEvent, SuggestionDecision,
};
use tauri::{AppHandle, Emitter, Manager, State};
use uuid::Uuid;

#[derive(Default)]
struct AppState {
    jobs: Mutex<HashMap<String, Arc<JobControl>>>,
}

#[derive(Default)]
struct JobControl {
    paused: AtomicBool,
    cancelled: AtomicBool,
}

fn document_session_id(document_path: &str) -> String {
    // 用 UUID v5 将“文档路径”稳定映射为 session id：
    // - 同一台机器上同一路径 => 同一个 id（用于恢复进度）
    // - 避免把路径直接当文件名（包含非法字符/过长）
    let namespace = Uuid::from_bytes([
        0x6c, 0x65, 0x73, 0x73, 0x61, 0x69, 0x2d, 0x64, 0x6f, 0x63, 0x2d, 0x6e, 0x73, 0x2d,
        0x30, 0x31,
    ]);
    Uuid::new_v5(&namespace, document_path.as_bytes()).to_string()
}

#[tauri::command]
fn load_settings(app: AppHandle) -> Result<AppSettings, String> {
    storage::load_settings(&app)
}

#[tauri::command]
fn save_settings(app: AppHandle, settings: AppSettings) -> Result<AppSettings, String> {
    storage::save_settings(&app, &settings)
}

#[tauri::command]
async fn test_provider(settings: AppSettings) -> Result<models::ProviderCheckResult, String> {
    rewrite::test_provider(&settings).await
}

#[tauri::command]
fn load_session(app: AppHandle, session_id: String) -> Result<DocumentSession, String> {
    storage::load_session(&app, &session_id)
}

#[tauri::command]
fn open_document(app: AppHandle, path: String) -> Result<DocumentSession, String> {
    if path.trim().is_empty() {
        return Err("文件路径不能为空。".to_string());
    }

    let canonical = fs::canonicalize(&path).map_err(|error| error.to_string())?;
    let canonical_str = canonical.to_string_lossy().to_string();
    let session_id = document_session_id(&canonical_str);

    if let Some(mut session) = storage::load_session_optional(&app, &session_id)? {
        // 进度恢复：如果上次崩溃/强退导致状态停留在 running/paused，这里统一降级，
        // 避免 UI 误以为还能继续后台任务（后台 job 在重启后不可恢复）。
        if matches!(session.status, RunningState::Running | RunningState::Paused) {
            session.status = RunningState::Cancelled;
            session.updated_at = Utc::now();
            storage::save_session(&app, &session)?;
        }
        return Ok(session);
    }

    let source_text = fs::read_to_string(&canonical).map_err(|error| error.to_string())?;
    if source_text.trim().is_empty() {
        return Err("文档内容为空。".to_string());
    }

    let settings = storage::load_settings(&app)?;
    let normalized_text = rewrite::normalize_text(&source_text);
    let chunks = rewrite::segment_text(&normalized_text, settings.chunk_preset)
        .into_iter()
        .enumerate()
        .map(|(index, chunk)| ChunkTask {
            index,
            source_text: chunk.text,
            separator_after: chunk.separator_after,
            status: ChunkStatus::Idle,
            error_message: None,
        })
        .collect::<Vec<_>>();

    let title = canonical
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("未命名文稿")
        .to_string();

    let now = Utc::now();
    let session = DocumentSession {
        id: session_id,
        title,
        document_path: canonical_str,
        source_text,
        normalized_text,
        chunks,
        suggestions: Vec::new(),
        next_suggestion_sequence: 1,
        status: RunningState::Idle,
        created_at: now,
        updated_at: now,
    };

    storage::save_session(&app, &session)?;
    Ok(session)
}

#[tauri::command]
async fn start_rewrite(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    mode: RewriteMode,
) -> Result<DocumentSession, String> {
    let session = storage::load_session(&app, &session_id)?;

    match mode {
        RewriteMode::Manual => run_manual_rewrite(&app, &session).await,
        RewriteMode::Auto => run_auto_rewrite(app, state, session),
    }
}

#[tauri::command]
fn pause_rewrite(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
) -> Result<DocumentSession, String> {
    let job = {
        let jobs = state
            .jobs
            .lock()
            .map_err(|_| "任务状态锁已损坏。".to_string())?;
        jobs.get(&session_id)
            .cloned()
            .ok_or_else(|| "当前没有可暂停的任务。".to_string())?
    };

    job.paused.store(true, Ordering::SeqCst);
    update_session_status(&app, &session_id, RunningState::Paused)
}

#[tauri::command]
fn resume_rewrite(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
) -> Result<DocumentSession, String> {
    let job = {
        let jobs = state
            .jobs
            .lock()
            .map_err(|_| "任务状态锁已损坏。".to_string())?;
        jobs.get(&session_id)
            .cloned()
            .ok_or_else(|| "当前没有可继续的任务。".to_string())?
    };

    job.paused.store(false, Ordering::SeqCst);
    update_session_status(&app, &session_id, RunningState::Running)
}

#[tauri::command]
fn cancel_rewrite(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
) -> Result<DocumentSession, String> {
    let maybe_job = {
        let jobs = state
            .jobs
            .lock()
            .map_err(|_| "任务状态锁已损坏。".to_string())?;
        jobs.get(&session_id).cloned()
    };

    if let Some(job) = maybe_job {
        job.cancelled.store(true, Ordering::SeqCst);
    }

    update_session_status(&app, &session_id, RunningState::Cancelled)
}

#[tauri::command]
fn apply_suggestion(
    app: AppHandle,
    session_id: String,
    suggestion_id: String,
) -> Result<DocumentSession, String> {
    let mut session = storage::load_session(&app, &session_id)?;
    let now = Utc::now();

    let (chunk_index, found) = session
        .suggestions
        .iter()
        .find(|item| item.id == suggestion_id)
        .map(|item| (item.chunk_index, true))
        .unwrap_or((0, false));

    if !found {
        return Err("未找到对应的修改对。".to_string());
    }

    for suggestion in session.suggestions.iter_mut() {
        if suggestion.chunk_index != chunk_index {
            continue;
        }

        if suggestion.id == suggestion_id {
            suggestion.decision = SuggestionDecision::Applied;
            suggestion.updated_at = now;
        } else if suggestion.decision == SuggestionDecision::Applied {
            suggestion.decision = SuggestionDecision::Dismissed;
            suggestion.updated_at = now;
        }
    }

    session.updated_at = now;
    storage::save_session(&app, &session)?;
    Ok(session)
}

#[tauri::command]
fn dismiss_suggestion(
    app: AppHandle,
    session_id: String,
    suggestion_id: String,
) -> Result<DocumentSession, String> {
    let mut session = storage::load_session(&app, &session_id)?;
    let now = Utc::now();
    let suggestion = session
        .suggestions
        .iter_mut()
        .find(|item| item.id == suggestion_id)
        .ok_or_else(|| "未找到对应的修改对。".to_string())?;

    suggestion.decision = SuggestionDecision::Dismissed;
    suggestion.updated_at = now;
    session.updated_at = now;
    storage::save_session(&app, &session)?;
    Ok(session)
}

#[tauri::command]
fn delete_suggestion(
    app: AppHandle,
    session_id: String,
    suggestion_id: String,
) -> Result<DocumentSession, String> {
    let mut session = storage::load_session(&app, &session_id)?;
    let now = Utc::now();

    let removed = session
        .suggestions
        .iter()
        .find(|item| item.id == suggestion_id)
        .map(|item| item.chunk_index);

    session.suggestions.retain(|item| item.id != suggestion_id);

    if let Some(chunk_index) = removed {
        let still_has_any = session
            .suggestions
            .iter()
            .any(|item| item.chunk_index == chunk_index);

        if !still_has_any {
            if let Some(chunk) = session.chunks.get_mut(chunk_index) {
                if chunk.status == ChunkStatus::Done {
                    chunk.status = ChunkStatus::Idle;
                }
            }
        }
    }

    session.updated_at = now;
    storage::save_session(&app, &session)?;
    Ok(session)
}

#[tauri::command]
async fn retry_chunk(
    app: AppHandle,
    session_id: String,
    index: usize,
) -> Result<DocumentSession, String> {
    process_chunk(&app, &session_id, index, false).await?;
    storage::load_session(&app, &session_id)
}

#[tauri::command]
fn export_document(app: AppHandle, session_id: String, path: String) -> Result<String, String> {
    let session = storage::load_session(&app, &session_id)?;
    let content = build_merged_text(&session);
    let path_buf = PathBuf::from(&path);

    if let Some(parent) = path_buf.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    fs::write(&path_buf, content).map_err(|error| error.to_string())?;
    Ok(path)
}

#[tauri::command]
fn finalize_document(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
) -> Result<String, String> {
    {
        // 避免与后台 job 竞争写 session 文件/源文件；如果任务仍在运行或退出中，直接拒绝。
        let jobs = state
            .jobs
            .lock()
            .map_err(|_| "任务状态锁已损坏。".to_string())?;
        if jobs.contains_key(&session_id) {
            return Err("后台任务仍在运行或正在退出，请稍后再试。".to_string());
        }
    }

    let session = storage::load_session(&app, &session_id)?;

    if matches!(session.status, RunningState::Running | RunningState::Paused) {
        return Err("当前文档正在执行自动任务，请先暂停并取消后再写回原文件。".to_string());
    }

    let content = build_merged_text(&session);
    let target = PathBuf::from(&session.document_path);

    // 保险起见：确保父目录存在（大多数情况下本来就存在）。
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    // 覆盖写回原文件：只写入“已应用”的修改，未应用的候选不会进入文件。
    fs::write(&target, content).map_err(|error| error.to_string())?;

    // 写回成功后再清理记录，避免“写失败但记录被删”的风险。
    storage::delete_session(&app, &session_id)?;

    Ok(session.document_path)
}

async fn run_manual_rewrite(
    app: &AppHandle,
    session: &DocumentSession,
) -> Result<DocumentSession, String> {
    if session.status == RunningState::Running || session.status == RunningState::Paused {
        return Err("当前文档正在执行自动任务，请先暂停或取消。".to_string());
    }

    let next_chunk = session
        .chunks
        .iter()
        .find(|chunk| matches!(chunk.status, ChunkStatus::Idle | ChunkStatus::Failed))
        .map(|chunk| chunk.index)
        .ok_or_else(|| "没有可继续处理的片段，当前文档可能已经全部完成。".to_string())?;

    process_chunk(app, &session.id, next_chunk, false).await?;
    storage::load_session(app, &session.id)
}

fn run_auto_rewrite(
    app: AppHandle,
    state: State<'_, AppState>,
    mut session: DocumentSession,
) -> Result<DocumentSession, String> {
    {
        let jobs = state
            .jobs
            .lock()
            .map_err(|_| "任务状态锁已损坏。".to_string())?;
        if jobs.contains_key(&session.id) {
            return Err("当前会话已经存在运行中的任务。".to_string());
        }
    }

    session.status = RunningState::Running;
    session.updated_at = Utc::now();
    storage::save_session(&app, &session)?;

    {
        let mut jobs = state
            .jobs
            .lock()
            .map_err(|_| "任务状态锁已损坏。".to_string())?;

        let job = Arc::new(JobControl::default());
        jobs.insert(session.id.clone(), job.clone());
        let session_id = session.id.clone();
        let app_handle = app.clone();

        tauri::async_runtime::spawn(async move {
            let result = run_auto_loop(app_handle.clone(), session_id.clone(), job.clone()).await;
            if let Err(error) = result {
                let _ = mark_session_failed(&app_handle, &session_id, error.clone());
                let _ = app_handle.emit(
                    "rewrite_failed",
                    RewriteFailedEvent {
                        session_id: session_id.clone(),
                        error,
                    },
                );
            }

            let state = app_handle.state::<AppState>();
            let _ = remove_job(&state, &session_id);
        });
    }

    Ok(session)
}

async fn run_auto_loop(
    app: AppHandle,
    session_id: String,
    job: Arc<JobControl>,
) -> Result<(), String> {
    let total = storage::load_session(&app, &session_id)?.chunks.len();

    for index in 0..total {
        loop {
            if job.cancelled.load(Ordering::SeqCst) {
                update_session_status(&app, &session_id, RunningState::Cancelled)?;
                app.emit(
                    "rewrite_finished",
                    SessionEvent {
                        session_id: session_id.clone(),
                    },
                )
                .map_err(|error| error.to_string())?;
                return Ok(());
            }

            if !job.paused.load(Ordering::SeqCst) {
                break;
            }

            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        }

        let current_session = storage::load_session(&app, &session_id)?;
        let chunk = current_session
            .chunks
            .get(index)
            .ok_or_else(|| "片段索引越界。".to_string())?;

        if chunk.status == ChunkStatus::Done {
            continue;
        }

        app.emit(
            "rewrite_progress",
            RewriteProgress {
                session_id: session_id.clone(),
                current_chunk: index + 1,
                total_chunks: total,
                mode: RewriteMode::Auto,
                running_state: RunningState::Running,
            },
        )
        .map_err(|error| error.to_string())?;

        process_chunk(&app, &session_id, index, true).await?;
    }

    let mut session = storage::load_session(&app, &session_id)?;
    session.status = compute_session_state(&session);
    session.updated_at = Utc::now();
    storage::save_session(&app, &session)?;
    app.emit(
        "rewrite_finished",
        SessionEvent {
            session_id: session.id.clone(),
        },
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

async fn process_chunk(
    app: &AppHandle,
    session_id: &str,
    index: usize,
    auto_approve: bool,
) -> Result<(), String> {
    mark_chunk_running(app, session_id, index)?;
    let settings = storage::load_settings(app)?;
    let session = storage::load_session(app, session_id)?;
    let source_text = session
        .chunks
        .get(index)
        .ok_or_else(|| "片段索引越界。".to_string())?
        .source_text
        .clone();

    match rewrite::rewrite_chunk(&settings, &source_text).await {
        Ok(candidate_text) => {
            let mut latest = storage::load_session(app, session_id)?;
            let chunk = latest
                .chunks
                .get_mut(index)
                .ok_or_else(|| "片段索引越界。".to_string())?;
            let now = Utc::now();
            let suggestion_id = Uuid::new_v4().to_string();
            let suggestion_sequence = latest.next_suggestion_sequence;
            latest.next_suggestion_sequence = latest.next_suggestion_sequence.saturating_add(1);

            let decision = if auto_approve {
                SuggestionDecision::Applied
            } else {
                SuggestionDecision::Proposed
            };

            if decision == SuggestionDecision::Applied {
                for suggestion in latest.suggestions.iter_mut() {
                    if suggestion.chunk_index == index
                        && suggestion.decision == SuggestionDecision::Applied
                    {
                        suggestion.decision = SuggestionDecision::Dismissed;
                        suggestion.updated_at = now;
                    }
                }
            }

            latest.suggestions.push(EditSuggestion {
                id: suggestion_id.clone(),
                sequence: suggestion_sequence,
                chunk_index: index,
                before_text: chunk.source_text.clone(),
                after_text: candidate_text.clone(),
                diff_spans: rewrite::build_diff(&chunk.source_text, &candidate_text),
                decision,
                created_at: now,
                updated_at: now,
            });
            chunk.status = ChunkStatus::Done;
            chunk.error_message = None;
            latest.updated_at = now;
            latest.status = if auto_approve {
                RunningState::Running
            } else {
                RunningState::Idle
            };
            storage::save_session(app, &latest)?;
            app.emit(
                "chunk_completed",
                ChunkCompletedEvent {
                    session_id: session_id.to_string(),
                    index,
                    suggestion_id,
                    suggestion_sequence,
                },
            )
            .map_err(|error| error.to_string())?;
            Ok(())
        }
        Err(error) => {
            let mut latest = storage::load_session(app, session_id)?;
            let chunk = latest
                .chunks
                .get_mut(index)
                .ok_or_else(|| "片段索引越界。".to_string())?;
            chunk.status = ChunkStatus::Failed;
            chunk.error_message = Some(error.clone());
            latest.updated_at = Utc::now();
            latest.status = RunningState::Failed;
            storage::save_session(app, &latest)?;
            Err(error)
        }
    }
}

fn mark_chunk_running(app: &AppHandle, session_id: &str, index: usize) -> Result<(), String> {
    let mut session = storage::load_session(app, session_id)?;
    let chunk = session
        .chunks
        .get_mut(index)
        .ok_or_else(|| "片段索引越界。".to_string())?;
    chunk.status = ChunkStatus::Running;
    chunk.error_message = None;
    session.updated_at = Utc::now();
    session.status = RunningState::Running;
    storage::save_session(app, &session)
}

fn update_session_status(
    app: &AppHandle,
    session_id: &str,
    status: RunningState,
) -> Result<DocumentSession, String> {
    let mut session = storage::load_session(app, session_id)?;
    session.status = status;
    session.updated_at = Utc::now();
    storage::save_session(app, &session)?;
    Ok(session)
}

fn mark_session_failed(app: &AppHandle, session_id: &str, error: String) -> Result<(), String> {
    let mut session = storage::load_session(app, session_id)?;
    session.status = RunningState::Failed;
    session.updated_at = Utc::now();
    if let Some(chunk) = session
        .chunks
        .iter_mut()
        .find(|chunk| chunk.status == ChunkStatus::Running)
    {
        chunk.status = ChunkStatus::Failed;
        chunk.error_message = Some(error);
    }
    storage::save_session(app, &session)
}

fn compute_session_state(session: &DocumentSession) -> RunningState {
    if session
        .chunks
        .iter()
        .any(|chunk| chunk.status == ChunkStatus::Failed)
    {
        return RunningState::Failed;
    }

    let all_done = session
        .chunks
        .iter()
        .all(|chunk| chunk.status == ChunkStatus::Done);

    if all_done {
        return RunningState::Completed;
    }

    RunningState::Idle
}

fn build_merged_text(session: &DocumentSession) -> String {
    let mut merged = String::new();

    for chunk in session.chunks.iter() {
        let applied = session
            .suggestions
            .iter()
            .filter(|item| {
                item.chunk_index == chunk.index && item.decision == SuggestionDecision::Applied
            })
            .max_by_key(|item| item.sequence);
        let body = applied
            .map(|item| item.after_text.as_str())
            .unwrap_or(chunk.source_text.as_str());

        merged.push_str(body);
        merged.push_str(&chunk.separator_after);
    }

    merged
}

fn remove_job(state: &AppState, session_id: &str) -> Result<(), String> {
    let mut jobs = state
        .jobs
        .lock()
        .map_err(|_| "任务状态锁已损坏。".to_string())?;
    jobs.remove(session_id);
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .manage(AppState::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            load_settings,
            save_settings,
            test_provider,
            open_document,
            load_session,
            start_rewrite,
            pause_rewrite,
            resume_rewrite,
            cancel_rewrite,
            apply_suggestion,
            dismiss_suggestion,
            delete_suggestion,
            retry_chunk,
            export_document,
            finalize_document
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
