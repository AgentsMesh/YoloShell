use crate::process::captured_tree::gone::{self, ProducersGoneRetirement};
use crate::process::inventory_test_support::TestProcess;
use crate::process::{CapturedProcessTree, PtySessionProof, ReportedProducersGoneRetirement};
use portable_pty::{PtySize, native_pty_system};
use std::os::unix::fs::PermissionsExt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use uuid::Uuid;

const TEST_CAPABILITY: &str = "reopen-seal-lifecycle";
const TEST_ROWS: u16 = 24;
const TEST_COLUMNS: u16 = 80;
const TEST_PIXEL_SIZE: u16 = 0;
const PERMISSION_BITS: u32 = (libc::S_IRWXU | libc::S_IRWXG | libc::S_IRWXO) as u32;
const SEALED_MODE: u32 = 0;
const MASTER_RELEASE_REQUIRED_DETAIL: &str = "sealed PTY retirement bypassed master release";
const RETIREMENT_REFROZE_DETAIL: &str = "sealed PTY retirement unexpectedly refroze";
const DIRECT_RETIREMENT_REQUIRED_DETAIL: &str =
    "no-local-alias PTY retirement required external master release";
const UNBOUND_RETIRED_DETAIL: &str = "unbound master-alias state prepared retirement";
const POST_COMPLETION_PANIC_DETAIL: &str = "injected post-completion retirement panic";

#[test]
fn master_release_completion_retries_after_post_completion_panic() {
    let process = TestProcess::spawn();
    let pair = open_pty();
    let path = pair.master.tty_name().unwrap();
    let mut proof =
        PtySessionProof::capture_bound(pair.master.as_ref(), TEST_CAPABILITY.into()).unwrap();
    proof.bind_runtime(Uuid::new_v4(), Uuid::new_v4()).unwrap();
    let release = proof.bind_master_alias_release(process.identity).unwrap();
    proof.seal_reopen_before_master_alias_close().unwrap();
    assert!(proof.has_required_pty_object_lease());
    assert_eq!(mode(&path), SEALED_MODE);
    let tree = CapturedProcessTree::capture_pty(process.identity, proof).unwrap();
    let (mut reported, ()) = gone::complete(tree).report(|_| Ok(())).unwrap();

    let ready = reported.prepare_retirement().unwrap();
    let mut pending = match reported.into_retirement(ready).unwrap() {
        ReportedProducersGoneRetirement::MasterRelease(pending) => pending,
        ReportedProducersGoneRetirement::Direct(_) => panic!("{MASTER_RELEASE_REQUIRED_DETAIL}"),
    };
    assert_eq!(mode(&path), SEALED_MODE);
    pending.panic_during_master_release_settlement(POST_COMPLETION_PANIC_DETAIL);
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            let _ = pending
                .prepare_retirement_after_master_release(Some(release.prove_closed_for_test()));
        }))
        .is_err()
    );
    let ready = pending
        .prepare_retirement_after_master_release(None)
        .unwrap();
    assert_eq!(mode(&path), SEALED_MODE);

    match pending
        .commit_retirement_after_master_release(ready)
        .unwrap()
    {
        ProducersGoneRetirement::Retired(reported) => assert!(reported.session_is_retired()),
        ProducersGoneRetirement::Refreeze(_) => panic!("{RETIREMENT_REFROZE_DETAIL}"),
    }
    assert_eq!(mode(&path), SEALED_MODE);
}

#[test]
fn repeated_no_alias_retirement_preparation_is_idempotent() {
    let process = TestProcess::spawn();
    let pair = open_pty();
    let path = pair.master.tty_name().unwrap();
    let mut proof =
        PtySessionProof::capture_bound(pair.master.as_ref(), TEST_CAPABILITY.into()).unwrap();
    proof.confirm_no_local_master_aliases().unwrap();
    let portable_pty::PtyPair { master, slave } = pair;
    drop(master);
    let _slave = slave;
    proof.seal_reopen_before_master_alias_close().unwrap();
    assert!(proof.has_required_pty_object_lease());
    let tree = CapturedProcessTree::capture_pty(process.identity, proof).unwrap();
    let (mut reported, ()) = gone::complete(tree).report(|_| Ok(())).unwrap();

    reported.prepare_retirement().unwrap();
    let ready = reported.prepare_retirement().unwrap();
    let retired = match reported.into_retirement(ready).unwrap() {
        ReportedProducersGoneRetirement::Direct(ready) => ready.commit(),
        ReportedProducersGoneRetirement::MasterRelease(_) => {
            panic!("{DIRECT_RETIREMENT_REQUIRED_DETAIL}")
        }
    };

    assert!(retired.session_is_retired());
    assert_eq!(mode(&path), SEALED_MODE);
}

#[test]
fn completed_process_discovery_completion_is_replayable() {
    let mut proof = PtySessionProof::discovery(TEST_CAPABILITY.to_owned()).unwrap();

    proof
        .complete_reopen_seal_after_master_alias_close()
        .unwrap();
    proof
        .complete_reopen_seal_after_master_alias_close()
        .unwrap();
}

#[test]
fn unbound_alias_state_is_rejected_before_retirement_preparation() {
    let process = TestProcess::spawn();
    let pair = open_pty();
    let proof =
        PtySessionProof::capture_bound(pair.master.as_ref(), TEST_CAPABILITY.into()).unwrap();
    let tree = CapturedProcessTree::capture_pty(process.identity, proof).unwrap();
    let (mut reported, ()) = gone::complete(tree).report(|_| Ok(())).unwrap();

    assert!(
        reported.prepare_retirement().is_err(),
        "{UNBOUND_RETIRED_DETAIL}"
    );
}

fn mode(path: &std::path::Path) -> u32 {
    std::fs::metadata(path).unwrap().permissions().mode() & PERMISSION_BITS
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
