use super::PtySessionProof;
use portable_pty::{PtySize, native_pty_system};

const TEST_CAPABILITY: &str = "bootstrap-fence-capability";
const TERMINAL_ROWS: u16 = 24;
const TERMINAL_COLUMNS: u16 = 80;
const TERMINAL_PIXEL_SIZE: u16 = 0;

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn only_bootstrap_capture_claims_the_exec_fence() {
    let pair = native_pty_system().openpty(pty_size()).unwrap();
    let generic =
        PtySessionProof::capture_bound(pair.master.as_ref(), TEST_CAPABILITY.to_owned()).unwrap();
    let mut bootstrap =
        PtySessionProof::capture_bootstrap_bound(pair.master.as_ref(), TEST_CAPABILITY.to_owned())
            .unwrap();

    assert!(!generic.bootstrap_exec_is_fenced());
    assert!(bootstrap.bootstrap_exec_is_fenced());
    assert!(generic.bootstrap_capability_scan_result().is_none());
    assert!(bootstrap.bootstrap_capability_scan_result().is_none());
    bootstrap.retain_bootstrap_capability_coverage();
    let cloned = bootstrap.clone();
    let scan = bootstrap.bootstrap_capability_scan_result().unwrap();
    assert_eq!(scan.coverage_status, super::CapabilityScanStatus::Pending);
    assert_eq!(scan.candidates.status, super::CapabilityScanStatus::Pending);
    assert!(scan.candidates.identities.is_empty());
    assert!(cloned.bootstrap_exec_is_fenced());
    assert!(cloned.bootstrap_capability_scan_result().is_none());
}

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn descriptor_installation_disarms_the_bootstrap_exec_fence() {
    let pair = native_pty_system().openpty(pty_size()).unwrap();
    let mut proof =
        PtySessionProof::capture_bootstrap_bound(pair.master.as_ref(), TEST_CAPABILITY.to_owned())
            .unwrap();
    let issuer = crate::process::MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, _writer) =
        crate::process::macos_process_capability::test_support::transfer(issuer).unwrap();

    proof.retain_bootstrap_capability_coverage();
    proof.install_process_capability(reader).unwrap();

    assert!(!proof.bootstrap_exec_is_fenced());
    assert!(proof.bootstrap_capability_scan_result().is_none());
}

fn pty_size() -> PtySize {
    PtySize {
        rows: TERMINAL_ROWS,
        cols: TERMINAL_COLUMNS,
        pixel_width: TERMINAL_PIXEL_SIZE,
        pixel_height: TERMINAL_PIXEL_SIZE,
    }
}
