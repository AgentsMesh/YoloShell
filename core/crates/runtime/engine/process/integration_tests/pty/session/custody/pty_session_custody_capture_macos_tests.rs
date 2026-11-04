use super::*;
use crate::process::MacosPassivePtyMaster;
use portable_pty::{PtySize, native_pty_system};

const TEST_CAPABILITY: &str = "recovered-custody-bootstrap";
const TERMINAL_ROWS: u16 = 24;
const TERMINAL_COLUMNS: u16 = 80;
const TERMINAL_PIXEL_SIZE: u16 = 0;

#[test]
fn recovered_custody_retains_confirmed_bootstrap_exec_evidence() {
    let (ordinary, mut recovered) = custody_proofs();

    assert!(!ordinary.bootstrap_exec_was_confirmed());
    assert!(recovered.bootstrap_exec_was_confirmed());
    assert!(recovered.bootstrap_exec_is_fenced());
    assert!(recovered.is_recovery_custody());

    recovered.retain_bootstrap_capability_coverage();
    let cloned = recovered.clone();
    assert!(recovered.bootstrap_capability_scan_result().is_some());
    assert!(cloned.is_recovery_custody());
    assert!(cloned.bootstrap_capability_scan_result().is_none());
}

#[test]
fn custody_scope_precedes_transferable_writer_for_non_released_states() {
    let (ordinary, recovered) = custody_proofs();
    assert_custody_scan_modes(&ordinary, &recovered);
}

fn assert_custody_scan_modes(ordinary: &PtySessionProof, recovered: &PtySessionProof) {
    use super::super::macos_liveness::{
        MacosCapabilityLivenessState as State, MacosGlobalScanMode as Mode, global_scan_mode,
    };

    for state in [
        State::Unarmed,
        State::WriterPresent,
        State::PrematurelyReleased,
        State::Indeterminate,
    ] {
        assert_eq!(global_scan_mode(ordinary, state), Mode::CustodyPtyObject);
        assert_eq!(
            global_scan_mode(recovered, state),
            Mode::RecoveryCustodyPtyObject
        );
    }
    for state in [State::ReleasedNeedsScan, State::ReleasedAfterRootExit] {
        assert_eq!(
            global_scan_mode(ordinary, state),
            Mode::ReleasedDiscoveryRetained
        );
        assert_eq!(
            global_scan_mode(recovered, state),
            Mode::ReleasedDiscoveryRetained
        );
    }
}

fn custody_proofs() -> (PtySessionProof, PtySessionProof) {
    let pair = native_pty_system().openpty(pty_size()).unwrap();
    let bound =
        PtySessionProof::capture_bound(pair.master.as_ref(), TEST_CAPABILITY.to_owned()).unwrap();
    let passive = MacosPassivePtyMaster::duplicate_exact(&bound).unwrap();
    let probe = crate::process::PtyMasterFdCustodyProbe::capture_for_test(&passive).unwrap();
    let issuer = crate::process::MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, _writer) = issuer.transfer_for_test().unwrap();

    let ordinary = PtySessionProof::capture_custody(probe.clone(), reader.clone()).unwrap();
    let recovered = PtySessionProof::capture_recovered_custody_for_test(probe, reader).unwrap();
    (ordinary, recovered)
}

fn pty_size() -> PtySize {
    PtySize {
        rows: TERMINAL_ROWS,
        cols: TERMINAL_COLUMNS,
        pixel_width: TERMINAL_PIXEL_SIZE,
        pixel_height: TERMINAL_PIXEL_SIZE,
    }
}
