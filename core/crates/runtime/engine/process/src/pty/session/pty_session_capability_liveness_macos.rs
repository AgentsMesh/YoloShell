use super::{CapabilityCandidates, CapabilityScanResult, CapabilityScanStatus, PtySessionProof};
use crate::process::{MacosProcessCapabilityObservation, MacosProcessCapabilityRelease};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum MacosCapabilityLivenessState {
    #[default]
    Unarmed,
    WriterPresent,
    ReleasedNeedsScan,
    ReleasedAfterRootExit,
    PrematurelyReleased,
    Indeterminate,
}

pub(super) enum MacosCapabilityLivenessDecision {
    Scan {
        state: MacosCapabilityLivenessState,
        require_pending: bool,
        reset_released: bool,
    },
    Complete(MacosCapabilityLivenessState),
    Pending(MacosCapabilityLivenessState),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MacosGlobalScanMode {
    LineageEnvironment,
    CustodyPtyObject,
    RecoveryCustodyPtyObject,
    PrematureBootstrapRelease,
    TransferableWriterPresent,
    TransferableWriterReleased,
    ReleasedDiscoveryRetained,
}

pub(super) fn global_scan_mode(
    proof: &PtySessionProof,
    state: MacosCapabilityLivenessState,
) -> MacosGlobalScanMode {
    if matches!(
        state,
        MacosCapabilityLivenessState::ReleasedNeedsScan
            | MacosCapabilityLivenessState::ReleasedAfterRootExit
    ) {
        return if proof.is_discovery() {
            MacosGlobalScanMode::ReleasedDiscoveryRetained
        } else if proof.has_transferable_descriptor() {
            MacosGlobalScanMode::TransferableWriterReleased
        } else {
            MacosGlobalScanMode::LineageEnvironment
        };
    }
    if proof.is_recovery_custody() {
        return MacosGlobalScanMode::RecoveryCustodyPtyObject;
    }
    if proof.is_custody_pty_object() {
        return MacosGlobalScanMode::CustodyPtyObject;
    }
    if !proof.has_transferable_descriptor() {
        return MacosGlobalScanMode::LineageEnvironment;
    }
    match state {
        MacosCapabilityLivenessState::PrematurelyReleased
            if proof.bootstrap_exec_was_confirmed() =>
        {
            MacosGlobalScanMode::PrematureBootstrapRelease
        }
        _ => MacosGlobalScanMode::TransferableWriterPresent,
    }
}

pub(super) fn observe(
    proof: &PtySessionProof,
    previous: MacosCapabilityLivenessState,
    root_exited: bool,
) -> MacosCapabilityLivenessDecision {
    let Some(reader) = proof
        .descriptor_capability
        .as_ref()
        .map(|authority| authority.process_reader())
    else {
        return MacosCapabilityLivenessDecision::Scan {
            state: MacosCapabilityLivenessState::Unarmed,
            require_pending: false,
            reset_released: false,
        };
    };
    let observation = reader.liveness_observation();
    if let Some(observation) = observation {
        return observed_by_monitor(
            observation,
            previous,
            root_exited || proof.is_discovery(),
            proof.bootstrap_exec_was_confirmed(),
        );
    }
    match previous {
        MacosCapabilityLivenessState::ReleasedAfterRootExit => {
            return MacosCapabilityLivenessDecision::Complete(previous);
        }
        MacosCapabilityLivenessState::ReleasedNeedsScan => {
            return MacosCapabilityLivenessDecision::Pending(previous);
        }
        MacosCapabilityLivenessState::PrematurelyReleased => {
            return premature_release_decision(proof.bootstrap_exec_was_confirmed());
        }
        MacosCapabilityLivenessState::Indeterminate => {
            return MacosCapabilityLivenessDecision::Pending(previous);
        }
        _ => {}
    }
    match reader.poll_release() {
        Ok(MacosProcessCapabilityRelease::WriterPresent) => MacosCapabilityLivenessDecision::Scan {
            state: MacosCapabilityLivenessState::WriterPresent,
            require_pending: root_exited || proof.is_discovery(),
            reset_released: false,
        },
        Ok(MacosProcessCapabilityRelease::Released) => {
            MacosCapabilityLivenessDecision::Pending(MacosCapabilityLivenessState::Indeterminate)
        }
        Err(_) => {
            MacosCapabilityLivenessDecision::Pending(MacosCapabilityLivenessState::Indeterminate)
        }
    }
}

pub(super) fn observed_by_monitor(
    observation: MacosProcessCapabilityObservation,
    previous: MacosCapabilityLivenessState,
    writer_requires_pending: bool,
    premature_fallback_authorized: bool,
) -> MacosCapabilityLivenessDecision {
    match previous {
        MacosCapabilityLivenessState::ReleasedAfterRootExit => {
            return MacosCapabilityLivenessDecision::Complete(previous);
        }
        MacosCapabilityLivenessState::ReleasedNeedsScan => {
            return MacosCapabilityLivenessDecision::Scan {
                state: previous,
                require_pending: false,
                reset_released: false,
            };
        }
        MacosCapabilityLivenessState::Indeterminate => {
            return MacosCapabilityLivenessDecision::Pending(previous);
        }
        MacosCapabilityLivenessState::PrematurelyReleased
            if observation != MacosProcessCapabilityObservation::Indeterminate =>
        {
            return premature_release_decision(premature_fallback_authorized);
        }
        _ => {}
    }
    match observation {
        MacosProcessCapabilityObservation::WriterPresent
        | MacosProcessCapabilityObservation::ReleasedPendingRootExit
        | MacosProcessCapabilityObservation::RootExited => MacosCapabilityLivenessDecision::Scan {
            state: MacosCapabilityLivenessState::WriterPresent,
            require_pending: writer_requires_pending,
            reset_released: false,
        },
        MacosProcessCapabilityObservation::ReleasedAfterRootExit => {
            if previous == MacosCapabilityLivenessState::ReleasedAfterRootExit {
                MacosCapabilityLivenessDecision::Complete(previous)
            } else {
                MacosCapabilityLivenessDecision::Scan {
                    state: MacosCapabilityLivenessState::ReleasedNeedsScan,
                    require_pending: previous != MacosCapabilityLivenessState::ReleasedNeedsScan,
                    reset_released: previous != MacosCapabilityLivenessState::ReleasedNeedsScan,
                }
            }
        }
        MacosProcessCapabilityObservation::PrematurelyReleased => {
            premature_release_decision(premature_fallback_authorized)
        }
        MacosProcessCapabilityObservation::Indeterminate => {
            MacosCapabilityLivenessDecision::Pending(MacosCapabilityLivenessState::Indeterminate)
        }
    }
}

include!("pty_session_capability_liveness_result_macos.rs");

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/session/pty_session_capability_liveness_macos_tests.rs"]
mod tests;
