use super::*;
use crate::process::PtySessionProof;
use std::path::PathBuf;

const PRIMARY_DEVICE_PATH: &str = "/dev/null";
const FOREIGN_DEVICE_PATH: &str = "/dev/zero";
const PRIMARY_CAPABILITY: &str = "primary-master-release-capability";
const FOREIGN_CAPABILITY: &str = "foreign-master-release-capability";
const PRIMARY_PID: u32 = 41_001;
const PRIMARY_START_TIME: u64 = 73_001;
const FOREIGN_PID: u32 = 41_002;
const FOREIGN_START_TIME: u64 = 73_002;
const PROOF_REJECTED_DETAIL: &str = "exact master-close authority proof was rejected";
const FOREIGN_PROOF_ACCEPTED_DETAIL: &str = "foreign master-close authority proof was accepted";
const SECOND_BIND_ACCEPTED_DETAIL: &str = "master-close authority was bound more than once";
const BOUND_CLONE_ACCEPTED_DETAIL: &str = "bound verifier authority was cloned";
const TREE_STATE_CHANGED_DETAIL: &str = "rejected proof changed the process-tree release state";

#[test]
fn same_fields_from_distinct_authorities_are_not_interchangeable() {
    let mut first = proof(PRIMARY_DEVICE_PATH, PRIMARY_CAPABILITY);
    let mut second = first.clone();
    let session = Uuid::new_v4();
    let epoch = Uuid::new_v4();
    first.bind_runtime(session, epoch).unwrap();
    second.bind_runtime(session, epoch).unwrap();
    let first_release = first.bind_master_alias_release(primary_root()).unwrap();
    let second_release = second.bind_master_alias_release(primary_root()).unwrap();

    let first_closed = first_release.prove_closed_for_test();
    assert!(
        first.validates_master_aliases_closed(&first_closed),
        "{PROOF_REJECTED_DETAIL}"
    );
    assert!(
        !second.validates_master_aliases_closed(&first_closed),
        "{FOREIGN_PROOF_ACCEPTED_DETAIL}"
    );
    assert!(
        !first.validates_master_aliases_closed(&second_release.prove_closed_for_test()),
        "{FOREIGN_PROOF_ACCEPTED_DETAIL}"
    );
}

#[test]
fn session_runtime_root_capability_and_object_mismatches_are_rejected() {
    let session = Uuid::new_v4();
    let epoch = Uuid::new_v4();
    let (target, target_release) = bound(
        PRIMARY_DEVICE_PATH,
        PRIMARY_CAPABILITY,
        session,
        epoch,
        primary_root(),
    );
    let foreign = [
        bound(
            PRIMARY_DEVICE_PATH,
            PRIMARY_CAPABILITY,
            Uuid::new_v4(),
            epoch,
            primary_root(),
        ),
        bound(
            PRIMARY_DEVICE_PATH,
            PRIMARY_CAPABILITY,
            session,
            Uuid::new_v4(),
            primary_root(),
        ),
        bound(
            PRIMARY_DEVICE_PATH,
            PRIMARY_CAPABILITY,
            session,
            epoch,
            foreign_root(),
        ),
        bound(
            PRIMARY_DEVICE_PATH,
            FOREIGN_CAPABILITY,
            session,
            epoch,
            primary_root(),
        ),
        bound(
            FOREIGN_DEVICE_PATH,
            PRIMARY_CAPABILITY,
            session,
            epoch,
            primary_root(),
        ),
    ];

    assert!(target.validates_master_aliases_closed(&target_release.prove_closed_for_test()));
    for (_, release) in foreign {
        assert!(
            !target.validates_master_aliases_closed(&release.prove_closed_for_test()),
            "{FOREIGN_PROOF_ACCEPTED_DETAIL}"
        );
    }
}

#[test]
fn verifier_can_only_bind_once_and_cannot_be_cloned_after_binding() {
    let mut target = proof(PRIMARY_DEVICE_PATH, PRIMARY_CAPABILITY);
    target.bind_runtime(Uuid::new_v4(), Uuid::new_v4()).unwrap();
    target.bind_master_alias_release(primary_root()).unwrap();
    assert!(
        target.bind_master_alias_release(primary_root()).is_err(),
        "{SECOND_BIND_ACCEPTED_DETAIL}"
    );
    let clone = std::panic::catch_unwind(|| target.clone());
    assert!(clone.is_err(), "{BOUND_CLONE_ACCEPTED_DETAIL}");
}

#[test]
fn process_tree_rejects_foreign_proof_without_changing_release_state() {
    let process = crate::process::inventory_test_support::TestProcess::spawn();
    let session = Uuid::new_v4();
    let epoch = Uuid::new_v4();
    let (target, target_release) = bound(
        PRIMARY_DEVICE_PATH,
        PRIMARY_CAPABILITY,
        session,
        epoch,
        process.identity,
    );
    let (_, foreign_release) = bound(
        PRIMARY_DEVICE_PATH,
        PRIMARY_CAPABILITY,
        session,
        epoch,
        process.identity,
    );
    let mut tree =
        crate::process::CapturedProcessTree::capture_pty(process.identity, target).unwrap();

    assert!(
        tree.mark_pty_master_released(foreign_release.prove_closed_for_test())
            .is_err()
    );
    assert!(!tree.pty_master_released(), "{TREE_STATE_CHANGED_DETAIL}");
    tree.mark_pty_master_released(target_release.prove_closed_for_test())
        .unwrap();
    assert!(tree.pty_master_released(), "{PROOF_REJECTED_DETAIL}");
}

fn bound(
    path: &str,
    capability: &str,
    session: Uuid,
    epoch: Uuid,
    root: ProcessIdentity,
) -> (PtySessionProof, PtyMasterAliasReleaseAuthority) {
    let mut target = proof(path, capability);
    target.bind_runtime(session, epoch).unwrap();
    let release = target.bind_master_alias_release(root).unwrap();
    (target, release)
}

fn proof(path: &str, capability: &str) -> PtySessionProof {
    PtySessionProof::capture(PathBuf::from(path), capability.to_owned()).unwrap()
}

fn primary_root() -> ProcessIdentity {
    ProcessIdentity {
        pid: PRIMARY_PID,
        start_time: PRIMARY_START_TIME,
    }
}

fn foreign_root() -> ProcessIdentity {
    ProcessIdentity {
        pid: FOREIGN_PID,
        start_time: FOREIGN_START_TIME,
    }
}
