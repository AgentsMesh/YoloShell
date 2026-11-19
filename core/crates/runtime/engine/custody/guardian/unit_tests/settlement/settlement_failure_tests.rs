use super::resolution::{
    INDETERMINATE_NAMESPACE_CONTEXT, NAMESPACE_LOST_CONTEXT, ORPHANED_RUNTIME_CONTEXT,
    resolve_verification,
};
use super::*;
use crate::namespace_lease::{NamespaceLease, NamespaceVerification};

const FAILURE_DETAIL: &str = "injected guardian settlement failure";
const HOME_NAME: &str = "guardian-home";
const DISPLACED_NAME: &str = "displaced-guardian-home";
const HOME_LABEL: &str = "guardian failure test home";
const INDETERMINATE_DETAIL: &str = "injected namespace verification failure";
const TEST_WORKER_PID: u32 = 48_101;
const TEST_WORKER_START: u64 = 58_101;
const TEST_CHILD_PID: u32 = 48_102;
const TEST_CHILD_START: u64 = 58_102;

#[test]
fn verified_namespace_retries_while_an_exact_runtime_process_is_live() {
    let (_root, _home, namespace) = fixture();
    let runtime = runtime();
    let _worker = relation(worker_identity(&runtime), ProcessIdentityRelation::Matching);
    let _child = relation(child_identity(&runtime), ProcessIdentityRelation::Gone);
    let mut log = ProgressFailureLog::default();

    assert!(matches!(
        resolve(
            &namespace,
            &runtime,
            anyhow::anyhow!(FAILURE_DETAIL),
            &mut log
        ),
        Resolution::Retry
    ));
    assert!(namespace.verify().is_ok());
}

#[test]
fn proven_namespace_loss_enters_guarded_process_settlement() {
    let (root, home, namespace) = fixture();
    let runtime = runtime();
    let _worker = relation(worker_identity(&runtime), ProcessIdentityRelation::Matching);
    let _child = relation(child_identity(&runtime), ProcessIdentityRelation::Matching);
    std::fs::rename(&home, root.path().join(DISPLACED_NAME)).unwrap();
    let mut log = ProgressFailureLog::default();

    let Resolution::RetainUntilSettled(error) = resolve(
        &namespace,
        &runtime,
        anyhow::anyhow!(FAILURE_DETAIL),
        &mut log,
    ) else {
        unreachable!();
    };

    assert!(error.to_string().contains(NAMESPACE_LOST_CONTEXT));
    assert!(namespace.verify().is_err());
}

#[test]
fn verified_progress_failure_contains_a_proven_gone_runtime() {
    let (_root, _home, namespace) = fixture();
    let runtime = runtime();
    let _worker = relation(worker_identity(&runtime), ProcessIdentityRelation::Gone);
    let _child = relation(child_identity(&runtime), ProcessIdentityRelation::Gone);
    let mut log = ProgressFailureLog::default();

    let Resolution::RetainUntilSettled(error) = resolve(
        &namespace,
        &runtime,
        anyhow::anyhow!(FAILURE_DETAIL),
        &mut log,
    ) else {
        unreachable!();
    };

    assert!(error.to_string().contains(ORPHANED_RUNTIME_CONTEXT));
}

#[test]
fn unknown_runtime_observation_retries_fail_closed() {
    let (_root, _home, namespace) = fixture();
    let runtime = runtime();
    let _worker = relation(worker_identity(&runtime), ProcessIdentityRelation::Gone);
    let _child = relation(child_identity(&runtime), ProcessIdentityRelation::Unknown);
    let mut log = ProgressFailureLog::default();

    assert!(matches!(
        resolve(
            &namespace,
            &runtime,
            anyhow::anyhow!(FAILURE_DETAIL),
            &mut log
        ),
        Resolution::Retry
    ));
}

#[test]
fn indeterminate_namespace_contains_a_proven_gone_runtime() {
    let runtime = runtime();
    let _worker = relation(worker_identity(&runtime), ProcessIdentityRelation::Gone);
    let _child = relation(child_identity(&runtime), ProcessIdentityRelation::Gone);
    let mut log = ProgressFailureLog::default();
    let verification = NamespaceVerification::Indeterminate(anyhow::anyhow!(INDETERMINATE_DETAIL));

    let Resolution::RetainUntilSettled(error) = resolve_verification(
        verification,
        &runtime,
        anyhow::anyhow!(FAILURE_DETAIL),
        &mut log,
    ) else {
        unreachable!();
    };

    assert!(error.to_string().contains(INDETERMINATE_NAMESPACE_CONTEXT));
}

fn fixture() -> (tempfile::TempDir, std::path::PathBuf, NamespaceLease) {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join(HOME_NAME);
    crate::paths::directory::PrivateDirectory::ensure(&home).unwrap();
    let namespace = NamespaceLease::capture(&[(HOME_LABEL, &home)]).unwrap();
    (root, home, namespace)
}

fn runtime() -> CustodyRuntimeIdentity {
    CustodyRuntimeIdentity {
        terminal_id: uuid::Uuid::new_v4(),
        runtime_epoch: uuid::Uuid::new_v4(),
        worker_id: uuid::Uuid::new_v4(),
        worker_pid: TEST_WORKER_PID,
        worker_start_time: TEST_WORKER_START,
        child_pid: TEST_CHILD_PID,
        child_start_time: TEST_CHILD_START,
        boot_id: crate::process::boot_id(),
    }
}

fn relation(
    identity: ProcessIdentity,
    relation: ProcessIdentityRelation,
) -> crate::process::ProcessIdentityRelationGuard {
    crate::process::install_test_process_identity_relation(
        identity.pid,
        identity.start_time,
        relation,
    )
}
