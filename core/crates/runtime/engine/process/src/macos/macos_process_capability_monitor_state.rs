use super::super::{MacosProcessCapabilityRelease, release::poll_descriptor_release};
use super::{MacosProcessCapabilityObservation, MonitorProgress};
use crate::process::LiveProcessIdentity;
use crate::process::macos_native::ProcessExitState;
use std::os::fd::{AsRawFd, OwnedFd};
use std::time::Instant;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RootReleaseState {
    MatchingRunning,
    IdentityGone,
    Exiting,
    Indeterminate,
}

pub(super) fn observe_kernel_root_exit(descriptor: &OwnedFd, progress: &mut MonitorProgress) {
    if is_terminal(progress.observation) {
        return;
    }
    let observation = match progress.observation {
        MacosProcessCapabilityObservation::ReleasedPendingRootExit => {
            MacosProcessCapabilityObservation::ReleasedAfterRootExit
        }
        MacosProcessCapabilityObservation::WriterPresent => {
            match poll_descriptor_release(descriptor.as_raw_fd()) {
                Ok(MacosProcessCapabilityRelease::WriterPresent) => {
                    MacosProcessCapabilityObservation::RootExited
                }
                Ok(MacosProcessCapabilityRelease::Released) => {
                    MacosProcessCapabilityObservation::ReleasedAfterRootExit
                }
                Err(_) => MacosProcessCapabilityObservation::Indeterminate,
            }
        }
        unchanged => unchanged,
    };
    transition(progress, observation);
}

pub(super) fn observe_descriptor(
    descriptor: &OwnedFd,
    root: &LiveProcessIdentity,
    progress: &mut MonitorProgress,
) {
    if is_terminal(progress.observation) {
        return;
    }
    let observation = match poll_descriptor_release(descriptor.as_raw_fd()) {
        Ok(MacosProcessCapabilityRelease::WriterPresent) => None,
        Ok(MacosProcessCapabilityRelease::Released)
            if progress.observation == MacosProcessCapabilityObservation::RootExited =>
        {
            Some(MacosProcessCapabilityObservation::ReleasedAfterRootExit)
        }
        Ok(MacosProcessCapabilityRelease::Released) => Some(release_observation(
            root,
            MacosProcessCapabilityObservation::ReleasedPendingRootExit,
        )),
        Err(_) => Some(MacosProcessCapabilityObservation::Indeterminate),
    };
    if let Some(observation) = observation {
        transition(progress, observation);
    }
}

pub(super) fn observe_release_deadline(root: &LiveProcessIdentity, progress: &mut MonitorProgress) {
    if progress.observation != MacosProcessCapabilityObservation::ReleasedPendingRootExit {
        return;
    }
    let observation =
        release_observation(root, MacosProcessCapabilityObservation::PrematurelyReleased);
    transition(progress, observation);
}

pub(super) fn reconcile_confirmed_root_exit(
    descriptor: &OwnedFd,
    root: &LiveProcessIdentity,
    progress: &mut MonitorProgress,
) {
    let observation = match progress.observation {
        MacosProcessCapabilityObservation::WriterPresent => {
            match poll_descriptor_release(descriptor.as_raw_fd()) {
                Ok(MacosProcessCapabilityRelease::WriterPresent) => {
                    MacosProcessCapabilityObservation::RootExited
                }
                Ok(MacosProcessCapabilityRelease::Released) => {
                    release_observation(root, MacosProcessCapabilityObservation::Indeterminate)
                }
                Err(_) => MacosProcessCapabilityObservation::Indeterminate,
            }
        }
        MacosProcessCapabilityObservation::RootExited => {
            match poll_descriptor_release(descriptor.as_raw_fd()) {
                Ok(MacosProcessCapabilityRelease::WriterPresent) => {
                    MacosProcessCapabilityObservation::RootExited
                }
                Ok(MacosProcessCapabilityRelease::Released) => {
                    MacosProcessCapabilityObservation::ReleasedAfterRootExit
                }
                Err(_) => MacosProcessCapabilityObservation::Indeterminate,
            }
        }
        MacosProcessCapabilityObservation::ReleasedPendingRootExit => {
            match poll_descriptor_release(descriptor.as_raw_fd()) {
                Ok(MacosProcessCapabilityRelease::Released) => release_observation(
                    root,
                    MacosProcessCapabilityObservation::PrematurelyReleased,
                ),
                Ok(MacosProcessCapabilityRelease::WriterPresent) | Err(_) => {
                    MacosProcessCapabilityObservation::Indeterminate
                }
            }
        }
        terminal => terminal,
    };
    transition(progress, observation);
}

fn release_observation(
    root: &LiveProcessIdentity,
    running: MacosProcessCapabilityObservation,
) -> MacosProcessCapabilityObservation {
    release_observation_from(root_release_state(root), running)
}

fn root_release_state(root: &LiveProcessIdentity) -> RootReleaseState {
    let state = match root.try_matches() {
        Ok(false) => RootReleaseState::IdentityGone,
        Ok(true) => match crate::process::macos_native::process_exit_state(root.pid()) {
            Ok(ProcessExitState::Running) => RootReleaseState::MatchingRunning,
            Ok(ProcessExitState::Exiting | ProcessExitState::MetadataMissing) => {
                RootReleaseState::Exiting
            }
            Err(_) => RootReleaseState::Indeterminate,
        },
        Err(_) => RootReleaseState::Indeterminate,
    };
    state
}

pub(super) fn release_observation_from(
    root: RootReleaseState,
    running: MacosProcessCapabilityObservation,
) -> MacosProcessCapabilityObservation {
    match root {
        RootReleaseState::MatchingRunning => running,
        RootReleaseState::IdentityGone | RootReleaseState::Exiting => {
            MacosProcessCapabilityObservation::ReleasedAfterRootExit
        }
        RootReleaseState::Indeterminate => MacosProcessCapabilityObservation::Indeterminate,
    }
}

pub(super) fn is_terminal(observation: MacosProcessCapabilityObservation) -> bool {
    matches!(
        observation,
        MacosProcessCapabilityObservation::ReleasedAfterRootExit
            | MacosProcessCapabilityObservation::PrematurelyReleased
            | MacosProcessCapabilityObservation::Indeterminate
    )
}

fn transition(progress: &mut MonitorProgress, observation: MacosProcessCapabilityObservation) {
    if observation == MacosProcessCapabilityObservation::ReleasedPendingRootExit {
        if progress.observation != observation {
            progress.release_deadline = Instant::now().checked_add(
                crate::session_timing::PROCESS_CAPABILITY_EXIT_EVENT_CONVERGENCE_TIMEOUT,
            );
            if progress.release_deadline.is_none() {
                progress.observation = MacosProcessCapabilityObservation::Indeterminate;
                return;
            }
        }
    } else {
        progress.release_deadline = None;
    }
    progress.observation = observation;
}
