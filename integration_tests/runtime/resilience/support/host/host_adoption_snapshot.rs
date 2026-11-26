use super::evaluate::{
    ADOPTION_RECORD_CHANGED_DETAIL, ADOPTION_VIEW_CHANGED_DETAIL, evaluate_observed_runtime,
};
use super::*;
use std::future::Future;
use yolo_runtime::TerminalView;

const ADOPTION_PUBLICATION_CHANGED_DETAIL: &str =
    "runtime publication advanced during Host adoption";
const ADOPTION_PREDECESSOR_COMPLETE_DETAIL: &str =
    "predecessor generation cannot complete adoption";

impl RuntimeAdoptionExpectation {
    fn observe_current(&self) -> Result<runtime_record_authority::BoundRuntimeRecord, String> {
        // Probe clones share the verified generation floor; the guard never crosses an RPC.
        let mut previous = self
            .observed
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let current = previous
            .refresh_current()
            .map_err(|error| format!("{ADOPTION_RECORD_CHANGED_DETAIL}: {error:#}"))?;
        current
            .record()
            .validate_update_from(previous.record())
            .map_err(|error| format!("{ADOPTION_RECORD_CHANGED_DETAIL}: {error}"))?;
        *previous = current.clone();
        Ok(current)
    }
}

pub(super) async fn inspect_runtime<Query, QueryFuture>(
    expected: &RuntimeAdoptionExpectation,
    successor_generation: u64,
    query: Query,
) -> Result<AdoptionProgress, String>
where
    Query: FnOnce() -> QueryFuture,
    QueryFuture: Future<Output = Result<TerminalView, ClientError>>,
{
    let before = expected.observe_current()?;
    if before.record().controller_generation != successor_generation {
        return evaluate_observed_runtime(expected, &before, None, successor_generation, || {
            unreachable!("{ADOPTION_PREDECESSOR_COMPLETE_DETAIL}")
        });
    }
    let view = query().await;
    let after = expected.observe_current()?;
    if after.record().controller_generation != successor_generation {
        return Err(ADOPTION_RECORD_CHANGED_DETAIL.into());
    }
    let view = match view {
        Ok(view) => view,
        Err(error) => {
            return Ok(super::wait::pending_session_view_error(
                before.record().terminal_id,
                error,
            ));
        }
    };
    if before.record() != after.record() {
        before
            .require_view_matches(&view)
            .or_else(|_| after.require_view_matches(&view))
            .map_err(|error| format!("{ADOPTION_VIEW_CHANGED_DETAIL}: {error:#}"))?;
        after
            .verify_runtime_file()
            .map_err(|error| format!("{ADOPTION_RECORD_CHANGED_DETAIL}: {error:#}"))?;
        return Ok(AdoptionProgress::Pending(format!(
            "{ADOPTION_PUBLICATION_CHANGED_DETAIL}: session={}",
            before.record().terminal_id,
        )));
    }
    evaluate_observed_runtime(expected, &after, Some(&view), successor_generation, || {
        yolo_process_test_support::observe_process_identity(
            after.record().worker_pid,
            after.record().worker_start_time,
        )
    })
}

#[cfg(test)]
#[path = "host_adoption_snapshot_tests.rs"]
mod tests;
