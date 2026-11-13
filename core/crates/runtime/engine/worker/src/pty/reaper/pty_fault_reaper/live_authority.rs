use super::FaultReap;
use super::constants::{
    PTY_FAULT_REAPER_RETRY_INITIAL_INTERVAL, PTY_FAULT_REAPER_RETRY_MAX_INTERVAL,
};
use crate::worker::pty_force_termination::ForceTerminationPoll;
use crate::worker::pty_missing_eof::MissingEofPoll;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::time::Duration;
#[path = "live_authority_initial_signal.rs"]
mod initial_signal;
#[path = "live_authority_observation.rs"]
mod observation;
#[path = "live_authority_progress.rs"]
mod progress;
pub(crate) use observation::observe_root_exit;
#[cfg(test)]
use observation::observe_waitable_root_exit;
#[cfg(test)]
pub(crate) use observation::retain_or_lose_wait_authority;
use progress::{finish_if_ready, poll_force, poll_output_deadline};
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../../../test_support/unit/pty_fault_reaper/completion_unwind_test_support.rs"]
mod completion_unwind_test_support;

pub(crate) fn reap(ownership: &mut FaultReap) {
    if let Err(payload) = reap_with_delays(
        ownership,
        PTY_FAULT_REAPER_RETRY_INITIAL_INTERVAL,
        PTY_FAULT_REAPER_RETRY_MAX_INTERVAL,
    ) {
        resume_unwind(payload);
    }
}

#[cfg(test)]
pub(crate) fn reap_without_delay(ownership: &mut FaultReap) {
    if let Err(payload) = reap_with_delays(ownership, Duration::ZERO, Duration::ZERO) {
        resume_unwind(payload);
    }
}
pub(crate) fn reap_with_delays(
    ownership: &mut FaultReap,
    initial_delay: Duration,
    max_delay: Duration,
) -> std::thread::Result<()> {
    let mut delay = initial_delay;
    let mut initial_signal_requested = false;
    loop {
        let finished = catch_unwind(AssertUnwindSafe(|| -> std::thread::Result<bool> {
            let Some(activity) = poll_reap(ownership, &mut initial_signal_requested)? else {
                return Ok(true);
            };
            publish_cleanup_progress(ownership, activity);
            delay = ownership
                .schedule
                .pause(delay, activity, initial_delay, max_delay);
            Ok(false)
        }))??;
        if finished {
            return Ok(());
        }
    }
}

pub(crate) fn poll_reap(
    ownership: &mut FaultReap,
    initial_signal_requested: &mut bool,
) -> std::thread::Result<Option<super::schedule::FaultReapActivity>> {
    let previous = ownership.progress();
    #[cfg(all(feature = "adversarial-test-hooks", not(test), target_os = "macos"))]
    ownership.publish_adversarial_diagnostic();
    observe_root_exit(ownership);
    if !*initial_signal_requested {
        initial_signal::request(ownership);
        *initial_signal_requested = true;
    }
    #[cfg(test)]
    publish_state(ownership);
    let bootstrap_pending = match super::bootstrap_failure::poll(ownership) {
        Ok(super::bootstrap_failure::BootstrapFailurePoll::NotApplicable) => false,
        Ok(super::bootstrap_failure::BootstrapFailurePoll::Pending) => true,
        Ok(super::bootstrap_failure::BootstrapFailurePoll::Retired) => {
            if finish_outcome(ownership)? {
                return Ok(None);
            }
            true
        }
        Err(error) => {
            ownership.remember(Err(error));
            true
        }
    };
    if bootstrap_pending {
        #[cfg(test)]
        publish_state(ownership);
        return Ok(Some(ownership.take_progress_after(previous)));
    }
    if ownership
        .custody()
        .processes
        .as_ref()
        .is_some_and(crate::worker::pty_process_custody::PtyProcessCustody::session_is_retired)
    {
        if finish_outcome(ownership)? {
            return Ok(None);
        }
        #[cfg(test)]
        publish_state(ownership);
        return Ok(Some(ownership.take_progress_after(previous)));
    }
    match poll_force(ownership) {
        Ok(ForceTerminationPoll::Complete) => {
            if finish_outcome(ownership)? {
                return Ok(None);
            }
        }
        Ok(ForceTerminationPoll::Unsupported(_proof)) => ownership.enter_observer_only(),
        Ok(ForceTerminationPoll::Pending) => match poll_output_deadline(ownership) {
            Ok(MissingEofPoll::Lost) => {
                if finish_outcome(ownership)? {
                    return Ok(None);
                }
            }
            Ok(MissingEofPoll::Pending) => {}
            Err(error) => ownership.remember(Err(error)),
        },
        Err(error) => ownership.remember(Err(error)),
    }
    #[cfg(test)]
    publish_state(ownership);
    let force_progressed = ownership.custody_mut().force.take_progress();
    Ok(Some(
        ownership
            .take_progress_after(previous)
            .with_semantic_progress(force_progressed),
    ))
}

pub(crate) fn finish_outcome(ownership: &mut FaultReap) -> std::thread::Result<bool> {
    // Propagate panic payloads as ordinary exits before unwinding outside the retry loop.
    catch_unwind(AssertUnwindSafe(|| {
        #[cfg(all(test, feature = "yolo-unit-tests"))]
        completion_unwind_test_support::before_completion(ownership);
        finish_if_ready(ownership)
    }))
}

pub(crate) fn publish_cleanup_progress(
    ownership: &FaultReap,
    activity: super::schedule::FaultReapActivity,
) {
    if activity == super::schedule::FaultReapActivity::SemanticProgress {
        ownership.publish_cleanup_progress();
    }
}

#[cfg(test)]
pub(crate) fn publish_state(ownership: &FaultReap) {
    ownership.publish_state_for_test(ownership.custody().force.output_fence_reached_for_test());
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../../unit_tests/pty_fault_reaper/live_authority_tests.rs"]
mod tests;
