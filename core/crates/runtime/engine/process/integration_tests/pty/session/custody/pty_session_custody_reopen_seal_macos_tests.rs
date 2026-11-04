use super::*;
use portable_pty::{PtySize, native_pty_system};
use std::os::unix::fs::PermissionsExt;

const TEST_CAPABILITY: &str = "custody-reopen-seal";
const TEST_ROWS: u16 = 24;
const TEST_COLUMNS: u16 = 80;
const TEST_PIXEL_SIZE: u16 = 0;
const PERMISSION_BITS: u32 = (libc::S_IRWXU | libc::S_IRWXG | libc::S_IRWXO) as u32;
const SEALED_MODE: u32 = 0;
const CUSTODY_CHANGE_ACCEPTED: &str = "changed Guardian custody reached destructive commit";

#[test]
fn custody_proof_seals_the_exact_slave_before_destructive_commit() {
    let fixture = fixture();
    let path = fixture.proof.tty_path.clone();
    let mut proof = fixture.probe.resilience_proof().unwrap();

    let authority = proof.acquire_reopen_seal().unwrap();
    assert_eq!(mode(&path), SEALED_MODE);
    proof.install_reopen_seal(authority).unwrap();
    proof.commit_reopen_seal_for_destruction().unwrap();

    assert_eq!(mode(&path), SEALED_MODE);
}

#[test]
fn custody_change_between_snapshot_and_commit_fails_closed() {
    let fixture = fixture();
    let path = fixture.proof.tty_path.clone();
    let original_mode = mode(&path);
    let mut proof = fixture.probe.resilience_proof().unwrap();
    let authority = proof.acquire_reopen_seal().unwrap();
    proof.install_reopen_seal(authority).unwrap();

    drop(fixture.passive);
    assert!(
        proof.commit_reopen_seal_for_destruction().is_err(),
        "{CUSTODY_CHANGE_ACCEPTED}"
    );
    drop(proof);

    assert_eq!(mode(&path), original_mode);
}

struct Fixture {
    _pair: portable_pty::PtyPair,
    proof: PtySessionProof,
    passive: MacosPassivePtyMaster,
    probe: crate::process::PtyMasterFdCustodyProbe,
}

fn fixture() -> Fixture {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: TEST_ROWS,
            cols: TEST_COLUMNS,
            pixel_width: TEST_PIXEL_SIZE,
            pixel_height: TEST_PIXEL_SIZE,
        })
        .unwrap();
    let proof =
        PtySessionProof::capture_bound(pair.master.as_ref(), TEST_CAPABILITY.to_owned()).unwrap();
    let passive = MacosPassivePtyMaster::duplicate_exact(&proof).unwrap();
    let probe = crate::process::PtyMasterFdCustodyProbe::capture_for_test(&passive).unwrap();
    Fixture {
        _pair: pair,
        proof,
        passive,
        probe,
    }
}

fn mode(path: &std::path::Path) -> u32 {
    std::fs::metadata(path).unwrap().permissions().mode() & PERMISSION_BITS
}
