use super::inventory_test_support::TestProcess;
use super::*;
use portable_pty::{PtySize, native_pty_system};
use std::sync::Arc;

const TEST_ROWS: u16 = 24;
const TEST_COLUMNS: u16 = 80;
const TEST_PIXEL_SIZE: u16 = 0;
const TEST_CAPABILITY: &str = "spawn-boundary-capture";

#[test]
fn pty_capture_in_spawn_boundary_never_reacquires_the_gate() {
    let root = TestProcess::spawn();
    let pair = test_pty();
    let proof =
        PtySessionProof::capture(pair.master.tty_name().unwrap(), TEST_CAPABILITY.to_owned())
            .unwrap();
    let spawn_guard = waitable_child_spawn_guard().unwrap();
    let _reject_reentry = reject_spawn_gate_entry_for_test();

    let tree = CapturedProcessTree::capture_pty_in_spawn_boundary(
        root.identity,
        proof,
        &spawn_guard.boundary(),
    )
    .unwrap();

    assert!(tree.events_are_gated_for_test());
}

#[test]
fn spawn_boundary_rejects_a_prebound_descriptor_capability_before_arming() {
    let root = TestProcess::spawn();
    let pair = test_pty();
    let mut proof =
        PtySessionProof::capture(pair.master.tty_name().unwrap(), TEST_CAPABILITY.to_owned())
            .unwrap();
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, _writer) = issuer.transfer_for_test().unwrap();
    proof
        .install_process_capability(Arc::clone(&reader))
        .unwrap();
    let spawn_guard = waitable_child_spawn_guard().unwrap();
    let _reject_reentry = reject_spawn_gate_entry_for_test();

    let error = CapturedProcessTree::capture_pty_in_spawn_boundary(
        root.identity,
        proof,
        &spawn_guard.boundary(),
    )
    .err()
    .unwrap();

    assert_eq!(error.to_string(), captured_tree::BOUND_PROOF_DETAIL);
    assert_eq!(reader.liveness_observation(), None);
}

fn test_pty() -> portable_pty::PtyPair {
    native_pty_system()
        .openpty(PtySize {
            rows: TEST_ROWS,
            cols: TEST_COLUMNS,
            pixel_width: TEST_PIXEL_SIZE,
            pixel_height: TEST_PIXEL_SIZE,
        })
        .unwrap()
}
