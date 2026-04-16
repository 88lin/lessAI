use std::sync::atomic::Ordering;

use tauri::AppHandle;

use crate::{
    models::{AppSettings, RewriteMode},
    rewrite_job_state::finalize_auto_session,
    state::{AppState, JobControl},
};

use super::auto_state::{finish_auto_loop, remove_in_flight_batch, AutoLoopStop, AutoTaskJoin};
use super::{auto_running_state, emit_rewrite_progress, snapshot_running_indices_from_batches};

enum AutoLoopFailure<'a> {
    Session,
    Batch(&'a [usize]),
}

pub(super) struct AutoLoopRuntime<'a> {
    app: &'a AppHandle,
    state: &'a AppState,
    session_id: &'a str,
    job: &'a JobControl,
    total_chunks: usize,
    max_concurrency: usize,
    completed_chunks: usize,
    tasks: tokio::task::JoinSet<AutoTaskJoin>,
    in_flight_batches: Vec<Vec<usize>>,
}

impl<'a> AutoLoopRuntime<'a> {
    pub(super) fn new(
        app: &'a AppHandle,
        state: &'a AppState,
        session_id: &'a str,
        job: &'a JobControl,
        max_concurrency: usize,
    ) -> Self {
        Self {
            app,
            state,
            session_id,
            job,
            total_chunks: 0,
            max_concurrency,
            completed_chunks: 0,
            tasks: tokio::task::JoinSet::new(),
            in_flight_batches: Vec::new(),
        }
    }

    pub(super) fn set_progress_baseline(&mut self, total_chunks: usize, completed_chunks: usize) {
        self.total_chunks = total_chunks;
        self.completed_chunks = completed_chunks;
    }

    pub(super) fn apply_settings(&mut self, settings: &AppSettings) {
        self.max_concurrency = settings.max_concurrency;
    }

    pub(super) fn emit_progress(&self) -> Result<(), String> {
        self.emit_current_progress()
    }

    pub(super) fn is_cancelled(&self) -> bool {
        self.job.cancelled.load(Ordering::SeqCst)
    }

    pub(super) fn is_paused(&self) -> bool {
        self.job.paused.load(Ordering::SeqCst)
    }

    pub(super) fn has_capacity(&self) -> bool {
        self.in_flight_batches.len() < self.max_concurrency
    }

    pub(super) fn has_in_flight_batches(&self) -> bool {
        !self.in_flight_batches.is_empty()
    }

    pub(super) fn start_batch<Spawn>(
        &mut self,
        batch_indices: Vec<usize>,
        spawn: Spawn,
    ) -> Result<(), String>
    where
        Spawn: FnOnce(&mut tokio::task::JoinSet<AutoTaskJoin>),
    {
        self.in_flight_batches.push(batch_indices);
        spawn(&mut self.tasks);
        self.emit_current_progress()
    }

    pub(super) fn remove_batch(&mut self, indices: &[usize]) -> Result<(), String> {
        remove_in_flight_batch(&mut self.in_flight_batches, indices)
    }

    pub(super) fn in_flight_batches(&self) -> &[Vec<usize>] {
        &self.in_flight_batches
    }

    pub(super) fn tasks_mut(&mut self) -> &mut tokio::task::JoinSet<AutoTaskJoin> {
        &mut self.tasks
    }

    pub(super) fn record_completed_progress(&mut self, chunk_count: usize) -> Result<(), String> {
        self.completed_chunks = self.completed_chunks.saturating_add(chunk_count);
        self.emit_current_progress()
    }

    pub(super) fn cancel(&mut self) -> Result<(), String> {
        self.finish(AutoLoopStop::Cancelled)
    }

    pub(super) fn session_result<T>(&mut self, result: Result<T, String>) -> Result<T, String> {
        self.resolve_result(result, AutoLoopFailure::Session)
    }

    pub(super) fn batch_result<T>(
        &mut self,
        indices: &[usize],
        result: Result<T, String>,
    ) -> Result<T, String> {
        self.resolve_result(result, AutoLoopFailure::Batch(indices))
    }

    fn finish(&mut self, stop: AutoLoopStop<'_>) -> Result<(), String> {
        finish_auto_loop(
            self.app,
            self.state,
            self.session_id,
            &mut self.tasks,
            &mut self.in_flight_batches,
            stop,
        )
    }

    pub(super) fn finish_successfully(&mut self) -> Result<(), String> {
        let result = finalize_auto_session(self.app, self.state, self.session_id).map(|_| ());
        self.session_result(result)
    }

    fn emit_current_progress(&self) -> Result<(), String> {
        emit_auto_progress(
            self.app,
            self.session_id,
            self.completed_chunks,
            &self.in_flight_batches,
            self.total_chunks,
            self.job,
            self.max_concurrency,
        )
    }

    fn resolve_result<T>(
        &mut self,
        result: Result<T, String>,
        failure: AutoLoopFailure<'_>,
    ) -> Result<T, String> {
        run_with_auto_failure(result, |error| match failure {
            AutoLoopFailure::Session => self.finish(AutoLoopStop::SessionFailed(error)),
            AutoLoopFailure::Batch(indices) => {
                self.finish(AutoLoopStop::BatchFailed { indices, error })
            }
        })
    }
}

fn run_with_auto_failure<T, Handle>(result: Result<T, String>, handle: Handle) -> Result<T, String>
where
    Handle: FnOnce(String) -> Result<(), String>,
{
    match result {
        Ok(value) => Ok(value),
        Err(error) => match handle(error) {
            Ok(()) => unreachable!("自动改写失败处理器不应返回成功"),
            Err(error) => Err(error),
        },
    }
}

fn emit_auto_progress(
    app: &AppHandle,
    session_id: &str,
    completed_chunks: usize,
    in_flight_batches: &[Vec<usize>],
    total_chunks: usize,
    job: &JobControl,
    max_concurrency: usize,
) -> Result<(), String> {
    emit_rewrite_progress(
        app,
        session_id,
        completed_chunks,
        snapshot_running_indices_from_batches(in_flight_batches),
        total_chunks,
        RewriteMode::Auto,
        auto_running_state(job),
        max_concurrency,
    )
}

#[cfg(test)]
#[path = "auto_runtime_tests.rs"]
mod tests;
