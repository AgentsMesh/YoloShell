use crate::process::captured_tree::gone::{self, ProducersGoneRetirement};
use crate::process::{CapturedProcessTree, DestructiveSessionUnavailable, PtySessionProof};
use support::{ChildGuard, TEST_CAPABILITY, pending};
use uuid::Uuid;

#[path = "captured_tree_retirement_authority_test_support.rs"]
mod support;

const MISMATCH_DETAIL: &str = "belongs to another pending closure";
const MASTER_MISMATCH_DETAIL: &str = "belongs to another process closure";
#[cfg(target_os = "macos")]
const MACOS_TEST_ROWS: u16 = 24;
#[cfg(target_os = "macos")]
const MACOS_TEST_COLUMNS: u16 = 80;
#[cfg(target_os = "macos")]
const MACOS_TEST_PIXEL_SIZE: u16 = 0;
const FOREIGN_PREPARATION_DETAIL: &str =
    "foreign master authority unexpectedly prepared retirement";
const FOREIGN_COMMIT_DETAIL: &str = "foreign retirement authority unexpectedly committed";
const LOCAL_REFREEZE_DETAIL: &str = "local retirement unexpectedly refroze";
const PLATFORM_PREPARATION_DETAIL: &str = "local retirement preparation failed unexpectedly";
#[cfg(target_os = "macos")]
const MACOS_MASTER_RELEASE_DETAIL: &str =
    "macOS producer retirement bypassed exact PTY master release";

#[test]
fn cross_pending_retirement_requires_exact_binding_and_local_settlement() {
    let child = ChildGuard::spawn();
    let session = Uuid::new_v4();
    let epoch = Uuid::new_v4();
    let (mut owner, owner_release) = pending(child.identity, session, epoch);
    let (mut foreign, foreign_release) = pending(child.identity, session, epoch);
    let (_, intruder_release) = pending(child.identity, session, epoch);
    assert_eq!(owner.root(), foreign.root());

    let error = match foreign
        .prepare_retirement_after_master_release(Some(intruder_release.prove_closed_for_test()))
    {
        Err(error) => error,
        Ok(_) => panic!("{FOREIGN_PREPARATION_DETAIL}"),
    };
    assert!(error.to_string().contains(MASTER_MISMATCH_DETAIL));
    assert!(!foreign.session_is_retired());

    let owner_ready =
        owner.prepare_retirement_after_master_release(Some(owner_release.prove_closed_for_test()));
    let owner_ready = match owner_ready {
        Ok(ready) => ready,
        Err(error)
            if cfg!(target_os = "linux")
                && error
                    .downcast_ref::<DestructiveSessionUnavailable>()
                    .is_some() =>
        {
            assert!(!owner.session_is_retired());
            return;
        }
        Err(error) => panic!("{PLATFORM_PREPARATION_DETAIL}: {error}"),
    };
    let failure = match foreign.commit_retirement_after_master_release(owner_ready) {
        Err(failure) => failure,
        Ok(_) => panic!("{FOREIGN_COMMIT_DETAIL}"),
    };
    assert!(failure.error().to_string().contains(MISMATCH_DETAIL));
    let mut foreign = failure.into_state();
    assert_eq!(foreign.root(), child.identity);
    assert!(!foreign.session_is_retired());

    let foreign_ready = foreign
        .prepare_retirement_after_master_release(Some(foreign_release.prove_closed_for_test()))
        .unwrap();
    assert_retired(
        foreign
            .commit_retirement_after_master_release(foreign_ready)
            .unwrap(),
    );

    let owner_ready = owner.prepare_retirement_after_master_release(None).unwrap();
    assert_retired(
        owner
            .commit_retirement_after_master_release(owner_ready)
            .unwrap(),
    );
}

#[cfg(target_os = "macos")]
#[test]
fn macos_retirement_drops_its_object_lease_before_host_master_release() {
    let child = ChildGuard::spawn();
    let pair = portable_pty::native_pty_system()
        .openpty(portable_pty::PtySize {
            rows: MACOS_TEST_ROWS,
            cols: MACOS_TEST_COLUMNS,
            pixel_width: MACOS_TEST_PIXEL_SIZE,
            pixel_height: MACOS_TEST_PIXEL_SIZE,
        })
        .unwrap();
    let mut proof =
        PtySessionProof::capture_bound(pair.master.as_ref(), TEST_CAPABILITY.to_owned()).unwrap();
    proof.bind_runtime(Uuid::new_v4(), Uuid::new_v4()).unwrap();
    let release = proof.bind_master_alias_release(child.identity).unwrap();
    let tree = CapturedProcessTree::capture_pty(child.identity, proof).unwrap();
    let tracked = gone::complete(tree);
    let (mut reported, ()) = tracked.report(|_| Ok(())).unwrap();
    assert!(reported.retains_local_object_lease_for_test());

    let retirement_ready = reported.prepare_retirement().unwrap();
    assert!(!reported.retains_local_object_lease_for_test());
    let mut pending = match reported.into_retirement(retirement_ready).unwrap() {
        gone::ReportedProducersGoneRetirement::MasterRelease(pending) => pending,
        gone::ReportedProducersGoneRetirement::Direct(_) => {
            panic!("{MACOS_MASTER_RELEASE_DETAIL}")
        }
    };
    let ready = pending
        .prepare_retirement_after_master_release(Some(release.prove_closed_for_test()))
        .unwrap();
    assert_retired(
        pending
            .commit_retirement_after_master_release(ready)
            .unwrap(),
    );
}

#[cfg(target_os = "macos")]
#[test]
fn macos_non_pty_retirement_remains_direct() {
    let child = ChildGuard::spawn();
    let tree = CapturedProcessTree::capture(child.identity).unwrap();
    let tracked = gone::complete(tree);
    let (reported, ()) = tracked.report(|_| Ok(())).unwrap();

    let retired = reported.retire_direct_for_test();
    assert!(retired.session_is_retired());
}

fn assert_retired(retirement: ProducersGoneRetirement) {
    match retirement {
        ProducersGoneRetirement::Retired(completion) => {
            assert!(completion.session_is_retired());
        }
        ProducersGoneRetirement::Refreeze(_) => panic!("{LOCAL_REFREEZE_DETAIL}"),
    }
}
