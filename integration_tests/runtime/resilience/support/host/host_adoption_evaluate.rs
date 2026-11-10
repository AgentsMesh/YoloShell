use super::*;
use yolo_process_test_support::ProcessIdentityObservation;
use yolo_runtime::model::TerminalView;

pub(super) const ADOPTION_RECORD_CHANGED_DETAIL: &str =
    "runtime identity changed during Host adoption";
pub(super) const ADOPTION_VIEW_CHANGED_DETAIL: &str =
    "runtime view changed identity during Host adoption";
const ADOPTION_WORKER_GONE_DETAIL: &str = "expected surviving Worker exited during Host adoption";
const ADOPTION_WORKER_UNKNOWN_DETAIL: &str =
    "expected surviving Worker identity became indeterminate during Host adoption";

#[cfg(test)]
pub(super) fn evaluate_runtime(
    expected: &RuntimeAdoptionExpectation,
    candidate: &RuntimeRecord,
    view: Option<&TerminalView>,
    successor_generation: u64,
) -> Result<AdoptionProgress, String> {
    evaluate_runtime_with_observer(expected, candidate, view, successor_generation, || {
        yolo_process_test_support::observe_process_identity(
            candidate.worker_pid,
            candidate.worker_start_time,
        )
    })
}

#[cfg(test)]
pub(super) fn evaluate_runtime_with_observation(
    expected: &RuntimeAdoptionExpectation,
    candidate: &RuntimeRecord,
    view: Option<&TerminalView>,
    successor_generation: u64,
    observation: ProcessIdentityObservation,
) -> Result<AdoptionProgress, String> {
    evaluate_runtime_with_observer(expected, candidate, view, successor_generation, || {
        observation
    })
}

#[cfg(test)]
fn evaluate_runtime_with_observer<F>(
    expected: &RuntimeAdoptionExpectation,
    candidate: &RuntimeRecord,
    view: Option<&TerminalView>,
    successor_generation: u64,
    observe_worker: F,
) -> Result<AdoptionProgress, String>
where
    F: FnOnce() -> ProcessIdentityObservation,
{
    let observed = expected
        .bound
        .refresh(candidate)
        .map_err(|error| format!("{ADOPTION_RECORD_CHANGED_DETAIL}: {error:#}"))?;
    evaluate_observed_runtime(
        expected,
        &observed,
        view,
        successor_generation,
        observe_worker,
    )
}

pub(super) fn evaluate_observed_runtime<F>(
    expected: &RuntimeAdoptionExpectation,
    observed: &runtime_record_authority::BoundRuntimeRecord,
    view: Option<&TerminalView>,
    successor_generation: u64,
    observe_worker: F,
) -> Result<AdoptionProgress, String>
where
    F: FnOnce() -> ProcessIdentityObservation,
{
    let candidate = observed.record();
    if candidate.controller_generation == expected.record().controller_generation {
        return Ok(pending(candidate));
    }
    if candidate.controller_generation != successor_generation {
        return Err(ADOPTION_RECORD_CHANGED_DETAIL.into());
    }
    observed
        .verify_runtime_file()
        .map_err(|error| format!("{ADOPTION_RECORD_CHANGED_DETAIL}: {error:#}"))?;
    let view = view.ok_or_else(|| ADOPTION_VIEW_CHANGED_DETAIL.to_owned())?;
    observed
        .require_view_matches(view)
        .map_err(|error| format!("{ADOPTION_VIEW_CHANGED_DETAIL}: {error:#}"))?;
    if view
        .runtime
        .as_ref()
        .is_some_and(|runtime| runtime.status == TerminalRuntimeStatus::Unreachable)
    {
        return Ok(pending(candidate));
    }
    match observe_worker() {
        ProcessIdentityObservation::Matching => {
            observed
                .verify_runtime_file()
                .map_err(|error| format!("{ADOPTION_RECORD_CHANGED_DETAIL}: {error:#}"))?;
            Ok(AdoptionProgress::Complete)
        }
        ProcessIdentityObservation::Gone => Err(ADOPTION_WORKER_GONE_DETAIL.into()),
        ProcessIdentityObservation::Unknown => Err(ADOPTION_WORKER_UNKNOWN_DETAIL.into()),
    }
}

fn pending(candidate: &RuntimeRecord) -> AdoptionProgress {
    AdoptionProgress::Pending(format!(
        "{ADOPTION_PENDING_DETAIL}: session={}",
        candidate.terminal_id
    ))
}
