use super::PtySessionProof;
use super::macos_liveness::{
    MacosCapabilityLivenessDecision, MacosCapabilityLivenessState, MacosGlobalScanMode,
    global_scan_mode, observed_by_monitor,
};
use crate::process::{MacosProcessCapabilityIssuer, MacosProcessCapabilityObservation};
use portable_pty::{PtySize, native_pty_system};

const TEST_CAPABILITY: &str = "confirmed-bootstrap-premature-release";
const TERMINAL_COLUMNS: u16 = 80;
const TERMINAL_ROWS: u16 = 24;
const TERMINAL_PIXEL_SIZE: u16 = 0;

#[test]
fn confirmed_bootstrap_exec_authorizes_environment_fallback_after_early_writer_release() {
    let pair = native_pty_system().openpty(pty_size()).unwrap();
    let mut proof =
        PtySessionProof::capture_bootstrap_bound(pair.master.as_ref(), TEST_CAPABILITY.to_owned())
            .unwrap();
    proof
        .take_bootstrap_exec_authority()
        .unwrap()
        .confirm_exec()
        .unwrap();
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, _writer) = issuer.transfer_for_test().unwrap();
    proof.install_process_capability(reader).unwrap();

    assert!(proof.bootstrap_exec_was_confirmed());
    assert!(matches!(
        observed_by_monitor(
            MacosProcessCapabilityObservation::PrematurelyReleased,
            MacosCapabilityLivenessState::WriterPresent,
            false,
            proof.bootstrap_exec_was_confirmed(),
        ),
        MacosCapabilityLivenessDecision::Scan {
            state: MacosCapabilityLivenessState::PrematurelyReleased,
            require_pending: false,
            reset_released: false,
        }
    ));
    assert_eq!(
        global_scan_mode(&proof, MacosCapabilityLivenessState::PrematurelyReleased),
        MacosGlobalScanMode::PrematureBootstrapRelease
    );
}

#[test]
fn unconfirmed_bootstrap_exec_keeps_early_writer_release_fail_closed() {
    assert!(matches!(
        observed_by_monitor(
            MacosProcessCapabilityObservation::PrematurelyReleased,
            MacosCapabilityLivenessState::WriterPresent,
            false,
            false,
        ),
        MacosCapabilityLivenessDecision::Pending(MacosCapabilityLivenessState::PrematurelyReleased)
    ));
}

#[test]
fn premature_release_cannot_be_reclassified_as_normal_root_exit() {
    assert!(matches!(
        observed_by_monitor(
            MacosProcessCapabilityObservation::ReleasedAfterRootExit,
            MacosCapabilityLivenessState::PrematurelyReleased,
            true,
            false,
        ),
        MacosCapabilityLivenessDecision::Pending(MacosCapabilityLivenessState::PrematurelyReleased)
    ));
}

#[test]
fn indeterminate_liveness_cannot_recover_into_bootstrap_fallback() {
    assert!(matches!(
        observed_by_monitor(
            MacosProcessCapabilityObservation::PrematurelyReleased,
            MacosCapabilityLivenessState::Indeterminate,
            true,
            true,
        ),
        MacosCapabilityLivenessDecision::Pending(MacosCapabilityLivenessState::Indeterminate)
    ));
}

fn pty_size() -> PtySize {
    PtySize {
        rows: TERMINAL_ROWS,
        cols: TERMINAL_COLUMNS,
        pixel_width: TERMINAL_PIXEL_SIZE,
        pixel_height: TERMINAL_PIXEL_SIZE,
    }
}
