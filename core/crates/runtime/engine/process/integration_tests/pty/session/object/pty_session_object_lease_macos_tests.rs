use super::*;
use portable_pty::{PtySize, native_pty_system};

const TEST_CAPABILITY: &str = "pty-object-lease-test";
const TEST_ROWS: u16 = 24;
const TEST_COLUMNS: u16 = 80;
const TEST_PIXEL_SIZE: u16 = 0;
const UNBOUND_PROOF_DETAIL: &str = "bound PTY proof lost its object lease";
const MISMATCH_ACCEPTED_DETAIL: &str = "mismatched PTY master was accepted";

#[test]
fn bound_capture_retains_matching_master_object() {
    let pair = open_pty();
    let proof =
        PtySessionProof::capture_bound(pair.master.as_ref(), TEST_CAPABILITY.to_owned()).unwrap();
    drop(pair.master);

    assert!(
        proof.has_required_pty_object_lease(),
        "{UNBOUND_PROOF_DETAIL}"
    );
}

#[test]
fn path_capture_has_no_master_object_lease() {
    let pair = open_pty();
    let proof =
        PtySessionProof::capture(pair.master.tty_name().unwrap(), TEST_CAPABILITY.to_owned())
            .unwrap();

    assert!(!proof.has_required_pty_object_lease());
}

#[test]
fn master_must_match_proof_slave_object() {
    let expected = open_pty();
    let other = open_pty();
    let proof = PtySessionProof::capture(
        expected.master.tty_name().unwrap(),
        TEST_CAPABILITY.to_owned(),
    )
    .unwrap();

    let error = MacosPtyObjectLease::capture(other.master.as_ref(), &proof)
        .err()
        .expect(MISMATCH_ACCEPTED_DETAIL);

    assert!(error.to_string().contains(MASTER_PATH_MISMATCH_DETAIL));
}

fn open_pty() -> portable_pty::PtyPair {
    native_pty_system()
        .openpty(PtySize {
            rows: TEST_ROWS,
            cols: TEST_COLUMNS,
            pixel_width: TEST_PIXEL_SIZE,
            pixel_height: TEST_PIXEL_SIZE,
        })
        .unwrap()
}
