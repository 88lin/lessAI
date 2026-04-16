mod auto;
mod auto_loop;
mod auto_runtime;
mod auto_state;
mod manual;
mod process;
mod support;

pub(crate) use auto::run_auto_rewrite;
pub(crate) use manual::run_manual_rewrite;
pub(crate) use process::process_chunk;
use process::process_loaded_chunk_batch;
#[cfg(test)]
use support::build_rewrite_source_snapshot;
use support::{
    auto_pending_queue, auto_running_state, collect_rewrite_batch_source_texts,
    emit_rewrite_finished, emit_rewrite_progress, load_rewriteable_session,
    load_rewriteable_session_for_active_job, next_manual_batch, prepare_auto_rewrite_session,
    prepare_loaded_rewrite_batch, rewrite_session_request, snapshot_running_indices_from_batches,
};

#[cfg(test)]
#[path = "rewrite_jobs_tests.rs"]
mod tests;
