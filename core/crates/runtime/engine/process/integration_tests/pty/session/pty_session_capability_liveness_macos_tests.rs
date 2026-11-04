use super::*;
use crate::process::MacosProcessCapabilityIssuer;
use crate::process::macos_process_capability::test_support;
use std::path::PathBuf;

const TEST_CAPABILITY: &str = "process-capability-liveness-test";
const READABLE_TTY: &str = "/dev/null";

#[test]
fn writer_presence_keeps_global_capability_discovery_armed() {
    let (proof, _writer) = proof_with_writer();
    assert!(matches!(
        observe(&proof, MacosCapabilityLivenessState::Unarmed, true,),
        MacosCapabilityLivenessDecision::Scan {
            state: MacosCapabilityLivenessState::WriterPresent,
            require_pending: true,
            reset_released: false,
        }
    ));
}

#[test]
fn unarmed_eof_after_root_exit_is_ordering_indeterminate() {
    let (proof, writer) = proof_with_writer();
    drop(writer);
    assert!(matches!(
        observe(&proof, MacosCapabilityLivenessState::WriterPresent, true,),
        MacosCapabilityLivenessDecision::Pending(MacosCapabilityLivenessState::Indeterminate)
    ));
}

#[test]
fn unarmed_eof_while_root_is_live_is_permanently_indeterminate() {
    let (proof, writer) = proof_with_writer();
    drop(writer);
    let first = observe(&proof, MacosCapabilityLivenessState::WriterPresent, false);
    assert!(matches!(
        first,
        MacosCapabilityLivenessDecision::Pending(MacosCapabilityLivenessState::Indeterminate)
    ));
    assert!(matches!(
        observe(&proof, MacosCapabilityLivenessState::Indeterminate, true,),
        MacosCapabilityLivenessDecision::Pending(MacosCapabilityLivenessState::Indeterminate)
    ));
}

#[test]
fn protocol_error_then_eof_remains_indeterminate() {
    let (proof, writer) = proof_with_writer();
    test_support::write_unexpected_byte(&writer).unwrap();
    assert!(matches!(
        observe(&proof, MacosCapabilityLivenessState::WriterPresent, true,),
        MacosCapabilityLivenessDecision::Pending(MacosCapabilityLivenessState::Indeterminate)
    ));
    drop(writer);
    assert!(matches!(
        observe(&proof, MacosCapabilityLivenessState::Indeterminate, true,),
        MacosCapabilityLivenessDecision::Pending(MacosCapabilityLivenessState::Indeterminate)
    ));
}

#[test]
fn monitored_eof_requires_a_fresh_authorized_sweep_before_completion() {
    assert!(matches!(
        observed_by_monitor(
            MacosProcessCapabilityObservation::ReleasedAfterRootExit,
            MacosCapabilityLivenessState::WriterPresent,
            true,
            false,
        ),
        MacosCapabilityLivenessDecision::Scan {
            state: MacosCapabilityLivenessState::ReleasedNeedsScan,
            require_pending: true,
            reset_released: true,
        }
    ));
    assert_eq!(
        complete_authorized_scan(
            MacosCapabilityLivenessState::ReleasedNeedsScan,
            CapabilityScanStatus::Complete,
        ),
        MacosCapabilityLivenessState::ReleasedAfterRootExit
    );
}

#[test]
fn bounded_sweep_keeps_its_progress_after_the_eof_transition() {
    assert!(matches!(
        observed_by_monitor(
            MacosProcessCapabilityObservation::ReleasedAfterRootExit,
            MacosCapabilityLivenessState::ReleasedNeedsScan,
            false,
            false,
        ),
        MacosCapabilityLivenessDecision::Scan {
            state: MacosCapabilityLivenessState::ReleasedNeedsScan,
            require_pending: false,
            reset_released: false,
        }
    ));
    assert_eq!(
        complete_authorized_scan(
            MacosCapabilityLivenessState::ReleasedNeedsScan,
            CapabilityScanStatus::Pending,
        ),
        MacosCapabilityLivenessState::ReleasedNeedsScan
    );
}

#[test]
fn unarmed_proof_selects_lineage_environment_mode() {
    let proof = PtySessionProof::discovery(TEST_CAPABILITY.to_owned()).unwrap();
    assert_eq!(
        global_scan_mode(&proof, MacosCapabilityLivenessState::Unarmed),
        MacosGlobalScanMode::LineageEnvironment
    );
}

#[test]
fn writer_presence_selects_transferable_present_mode() {
    let (proof, _writer) = proof_with_writer();
    assert_eq!(
        global_scan_mode(&proof, MacosCapabilityLivenessState::WriterPresent),
        MacosGlobalScanMode::TransferableWriterPresent
    );
}

#[test]
fn released_discovery_selects_retained_identity_mode() {
    let (proof, _writer) = proof_with_writer();
    for state in [
        MacosCapabilityLivenessState::ReleasedNeedsScan,
        MacosCapabilityLivenessState::ReleasedAfterRootExit,
    ] {
        assert_eq!(
            global_scan_mode(&proof, state),
            MacosGlobalScanMode::ReleasedDiscoveryRetained
        );
    }
}

#[test]
fn released_pty_selects_full_descriptor_mode() {
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, _writer) = test_support::transfer(issuer).unwrap();
    let mut proof =
        PtySessionProof::capture(PathBuf::from(READABLE_TTY), TEST_CAPABILITY.to_owned()).unwrap();
    proof.install_process_capability(reader).unwrap();

    assert_eq!(
        global_scan_mode(&proof, MacosCapabilityLivenessState::ReleasedNeedsScan),
        MacosGlobalScanMode::TransferableWriterReleased
    );
}

#[test]
fn released_mode_cannot_regress_on_a_stale_root_exited_observation() {
    for previous in [
        MacosCapabilityLivenessState::ReleasedNeedsScan,
        MacosCapabilityLivenessState::ReleasedAfterRootExit,
    ] {
        let decision = observed_by_monitor(
            MacosProcessCapabilityObservation::RootExited,
            previous,
            true,
            false,
        );
        match previous {
            MacosCapabilityLivenessState::ReleasedNeedsScan => assert!(matches!(
                decision,
                MacosCapabilityLivenessDecision::Scan {
                    state: MacosCapabilityLivenessState::ReleasedNeedsScan,
                    require_pending: false,
                    reset_released: false,
                }
            )),
            MacosCapabilityLivenessState::ReleasedAfterRootExit => assert!(matches!(
                decision,
                MacosCapabilityLivenessDecision::Complete(
                    MacosCapabilityLivenessState::ReleasedAfterRootExit
                )
            )),
            _ => unreachable!(),
        }
    }
}

fn proof_with_writer() -> (
    PtySessionProof,
    crate::process::MacosProcessCapabilityChildWriter,
) {
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, writer) = test_support::transfer(issuer).unwrap();
    let mut proof = PtySessionProof::discovery(TEST_CAPABILITY.to_owned()).unwrap();
    proof.install_process_capability(reader).unwrap();
    (proof, writer)
}
