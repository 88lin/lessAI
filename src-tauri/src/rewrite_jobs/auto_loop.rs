use std::{collections::HashSet, sync::Arc};

use tauri::{AppHandle, Manager};

use crate::{
    rewrite,
    rewrite_job_state::{clear_running_chunks, mark_chunks_running},
    session_access::CurrentSessionRequest,
    session_edit::{mutate_session_now, save_session_value, SessionMutation},
    state::{AppState, JobControl},
    storage,
};

use super::auto_runtime::AutoLoopRuntime;
use super::auto_state::{commit_auto_batch, ensure_in_flight_batches_drained};
use super::{
    collect_rewrite_batch_source_texts, load_rewriteable_session_for_active_job,
    prepare_auto_rewrite_session,
};

const AUTO_LOOP_UNINITIALIZED_CONCURRENCY: usize = 0;

trait BatchSettlement {
    fn remove_batch_checked(&mut self, indices: &[usize]) -> Result<(), String>;
    fn apply_batch_result<T>(
        &mut self,
        indices: &[usize],
        result: Result<T, String>,
    ) -> Result<T, String>;
    fn record_completed_checked(&mut self, completed_count: usize) -> Result<(), String>;
}

impl BatchSettlement for AutoLoopRuntime<'_> {
    fn remove_batch_checked(&mut self, indices: &[usize]) -> Result<(), String> {
        let remove_result = self.remove_batch(indices);
        self.session_result(remove_result)
    }

    fn apply_batch_result<T>(
        &mut self,
        indices: &[usize],
        result: Result<T, String>,
    ) -> Result<T, String> {
        self.batch_result(indices, result)
    }

    fn record_completed_checked(&mut self, completed_count: usize) -> Result<(), String> {
        let progress_result = self.record_completed_progress(completed_count);
        self.session_result(progress_result)
    }
}

pub(super) async fn run_auto_loop(
    app: AppHandle,
    session_id: String,
    job: Arc<JobControl>,
    target_indices: Option<HashSet<usize>>,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    let app_state = state.inner();
    let mut runtime = AutoLoopRuntime::new(
        &app,
        app_state,
        &session_id,
        &job,
        AUTO_LOOP_UNINITIALIZED_CONCURRENCY,
    );
    let settings = runtime.session_result(storage::load_settings(&app))?;
    runtime.apply_settings(&settings);
    let ensure_rewriteable_session =
        || load_rewriteable_session_for_active_job(&app, app_state, &session_id);
    runtime.session_result(ensure_rewriteable_session())?;

    let chunks_per_request = settings.chunks_per_request;
    let client = Arc::new(runtime.session_result(rewrite::build_client(&settings))?);

    let (format, total_chunks, mut pending, source_snapshot, completed_chunks) = runtime
        .session_result(mutate_session_now(
            CurrentSessionRequest::stored(&app, app_state, &session_id),
            |session, now| {
                let touched = clear_running_chunks(session);
                let prepared = prepare_auto_rewrite_session(session, target_indices.as_ref());
                let result = (
                    prepared.format,
                    prepared.total_chunks,
                    prepared.pending,
                    prepared.source_snapshot,
                    prepared.completed_chunks,
                );

                if touched {
                    return Ok(save_session_value(session, now, result));
                }

                Ok(SessionMutation::SkipSave(result))
            },
        ))?;

    runtime.set_progress_baseline(total_chunks, completed_chunks);
    runtime.session_result(runtime.emit_progress())?;

    if pending.is_empty() {
        return runtime.finish_successfully();
    }

    loop {
        if runtime.is_cancelled() {
            return runtime.cancel();
        }

        while !runtime.is_paused() && runtime.has_capacity() {
            let batch_indices =
                crate::rewrite_targets::take_next_auto_batch(&mut pending, chunks_per_request);
            if batch_indices.is_empty() {
                break;
            }

            runtime.session_result(ensure_rewriteable_session())?;
            let batch_source_texts = runtime.session_result(collect_rewrite_batch_source_texts(
                &source_snapshot,
                &batch_indices,
            ))?;
            runtime.session_result(mark_chunks_running(
                &app,
                app_state,
                &session_id,
                &batch_indices,
            ))?;
            let batch_client = client.clone();
            let batch_settings = settings.clone();
            let batch_format = format;
            let start_batch_result = runtime.start_batch(batch_indices.clone(), |tasks| {
                tasks.spawn(async move {
                    let result = rewrite::rewrite_chunks_with_client(
                        &batch_client,
                        &batch_settings,
                        &batch_source_texts,
                        batch_format,
                    )
                    .await;
                    (batch_indices, result)
                });
            });
            runtime.session_result(start_batch_result)?;
        }

        if pending.is_empty() && !runtime.has_in_flight_batches() {
            break;
        }

        if !runtime.has_in_flight_batches() {
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            continue;
        }

        match tokio::time::timeout(
            std::time::Duration::from_millis(250),
            runtime.tasks_mut().join_next(),
        )
        .await
        {
            Ok(Some(joined)) => match joined {
                Ok((indices, Ok(candidate_texts))) => {
                    finish_completed_batch_steps(
                        &mut runtime,
                        &indices,
                        |runtime: &mut AutoLoopRuntime<'_>| {
                            let completed_batch = runtime.apply_batch_result(
                                &indices,
                                commit_auto_batch(
                                    &app,
                                    app_state,
                                    &session_id,
                                    &indices,
                                    candidate_texts,
                                ),
                            )?;
                            Ok(completed_batch.len())
                        },
                    )?;
                }
                Ok((indices, Err(error))) => {
                    return finish_failed_batch_steps(&mut runtime, &indices, error);
                }
                Err(join_error) => {
                    return runtime.session_result(Err(format!("后台任务异常退出：{join_error}")));
                }
            },
            Ok(None) => {
                let error = ensure_in_flight_batches_drained(runtime.in_flight_batches())
                    .expect_err(
                        "join set drained branch should only occur when in-flight batches remain",
                    );
                return runtime.session_result(Err(error));
            }
            Err(_) => {}
        }
    }

    runtime.finish_successfully()
}

fn finish_completed_batch_steps<S, Commit>(
    settlement: &mut S,
    indices: &[usize],
    commit: Commit,
) -> Result<(), String>
where
    S: BatchSettlement,
    Commit: FnOnce(&mut S) -> Result<usize, String>,
{
    settlement.remove_batch_checked(indices)?;
    let completed_count = commit(settlement)?;
    settlement.record_completed_checked(completed_count)
}

fn finish_failed_batch_steps<S, T>(
    settlement: &mut S,
    indices: &[usize],
    error: String,
) -> Result<T, String>
where
    S: BatchSettlement,
{
    settlement.remove_batch_checked(indices)?;
    settlement.apply_batch_result(indices, Err(error))
}

#[cfg(test)]
#[path = "auto_loop_tests.rs"]
mod tests;
