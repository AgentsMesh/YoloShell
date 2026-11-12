use crate::process::captured_tree::gone::{
    self, ProducersGoneRetirement, ReportedProducersGoneRetirement,
};
use crate::process::inventory_test_support::TestProcess;
use crate::process::{CapturedProcessTree, PtyMasterAliasReleaseAuthority, PtySessionProof};
use portable_pty::{PtyPair, PtySize, native_pty_system};
use std::fs::OpenOptions;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use uuid::Uuid;

const TEST_CAPABILITY: &str = "natural-retirement-reopen-seal";
const TEST_ROWS: u16 = 24;
const TEST_COLUMNS: u16 = 80;
const TEST_PIXEL_SIZE: u16 = 0;
const SEALED_MODE: u32 = 0;
const PERMISSION_BITS: u32 = (libc::S_IRWXU | libc::S_IRWXG | libc::S_IRWXO) as u32;
const SLAVE_OPEN_FLAGS: libc::c_int =
    libc::O_NOCTTY | libc::O_NONBLOCK | libc::O_CLOEXEC | libc::O_NOFOLLOW;
const FAILURE_DROPPED_LEASE_DETAIL: &str =
    "failed natural retirement dropped exact PTY object custody";
const MASTER_RELEASE_REQUIRED_DETAIL: &str = "natural PTY retirement bypassed exact master release";
const REOPEN_ALLOWED_DETAIL: &str = "natural PTY retirement left slave reopen enabled";
const RETIREMENT_REFROZE_DETAIL: &str = "natural PTY retirement unexpectedly refroze";
const UNSEALED_MODE_DETAIL: &str = "test PTY started with an already sealed slave";
const MISSING_SEAL_ACCEPTED_DETAIL: &str = "PTY lifecycle accepted missing reopen-seal custody";

#[test]
fn partial_seal_failure_retains_lease_and_blocks_reopen_until_retry() {
    let Fixture {
        _process,
        _pty,
        path,
        release,
        mut reported,
    } = Fixture::new();
    let original_mode = mode(&path);
    assert_ne!(original_mode, SEALED_MODE, "{UNSEALED_MODE_DETAIL}");
    crate::process::pty_session_proof::fail_next_reopen_seal_validation_for_test();

    assert!(reported.prepare_retirement().is_err());
    assert!(
        reported.retains_local_object_lease_for_test(),
        "{FAILURE_DROPPED_LEASE_DETAIL}"
    );
    assert_eq!(mode(&path), SEALED_MODE);
    assert!(reopen(&path).is_err(), "{REOPEN_ALLOWED_DETAIL}");

    let ready = reported.prepare_retirement().unwrap();
    assert!(!reported.retains_local_object_lease_for_test());
    assert_eq!(mode(&path), SEALED_MODE);
    assert!(reopen(&path).is_err(), "{REOPEN_ALLOWED_DETAIL}");
    let mut pending = match reported.into_retirement(ready).unwrap() {
        ReportedProducersGoneRetirement::MasterRelease(pending) => pending,
        ReportedProducersGoneRetirement::Direct(_) => panic!("{MASTER_RELEASE_REQUIRED_DETAIL}"),
    };
    let ready = pending
        .prepare_retirement_after_master_release(Some(release.prove_closed_for_test()))
        .unwrap();
    match pending
        .commit_retirement_after_master_release(ready)
        .unwrap()
    {
        ProducersGoneRetirement::Retired(reported) => assert!(reported.session_is_retired()),
        ProducersGoneRetirement::Refreeze(_) => panic!("{RETIREMENT_REFROZE_DETAIL}"),
    }
}

#[test]
fn idle_pty_completion_remains_fail_closed() {
    let process = TestProcess::spawn();
    let pair = open_pty();
    let mut proof =
        PtySessionProof::capture_bound(pair.master.as_ref(), TEST_CAPABILITY.into()).unwrap();
    proof.bind_runtime(Uuid::new_v4(), Uuid::new_v4()).unwrap();
    let _release = proof.bind_master_alias_release(process.identity).unwrap();

    assert!(
        proof.commit_reopen_seal_for_destruction().is_err(),
        "{MISSING_SEAL_ACCEPTED_DETAIL}"
    );
    assert!(
        proof
            .complete_reopen_seal_after_master_alias_close()
            .is_err(),
        "{MISSING_SEAL_ACCEPTED_DETAIL}"
    );
}

struct Fixture {
    _process: TestProcess,
    _pty: PtyPair,
    path: PathBuf,
    release: PtyMasterAliasReleaseAuthority,
    reported: crate::process::ReportedProducersGone,
}

impl Fixture {
    fn new() -> Self {
        let mut process = TestProcess::spawn();
        let pair = open_pty();
        let path = pair.master.tty_name().unwrap();
        let mut proof =
            PtySessionProof::capture_bound(pair.master.as_ref(), TEST_CAPABILITY.into()).unwrap();
        proof.bind_runtime(Uuid::new_v4(), Uuid::new_v4()).unwrap();
        let release = proof.bind_master_alias_release(process.identity).unwrap();
        let tree = CapturedProcessTree::capture_pty(process.identity, proof).unwrap();
        process.terminate();
        let reported = gone::complete(tree).report(|_| Ok(())).unwrap().0;
        Self {
            _process: process,
            _pty: pair,
            path,
            release,
            reported,
        }
    }
}

fn open_pty() -> PtyPair {
    native_pty_system()
        .openpty(PtySize {
            rows: TEST_ROWS,
            cols: TEST_COLUMNS,
            pixel_width: TEST_PIXEL_SIZE,
            pixel_height: TEST_PIXEL_SIZE,
        })
        .unwrap()
}

fn mode(path: &Path) -> u32 {
    std::fs::metadata(path).unwrap().permissions().mode() & PERMISSION_BITS
}

fn reopen(path: &Path) -> std::io::Result<std::fs::File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(SLAVE_OPEN_FLAGS)
        .open(path)
}
