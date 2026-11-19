use super::{
    model::{Configuration, Inspection},
    *,
};
use yolo_release_gate_system::{LINUX_AMD64_PROFILE, linux_gate_command};
#[path = "../test_support/container_engine.rs"]
mod fixture;
use fixture::*;
#[path = "container_admission.rs"]
mod admission_tests;
#[path = "container_cid.rs"]
mod cid_tests;
#[path = "container_resources.rs"]
mod resource_tests;

const BAZEL_PATH: &str = "/fixture/bazel";
const USER: &str = "501:501";
const FOREIGN_OWNER: &str = "foreign-owner";
const FOREIGN_MOUNT: &str = "/foreign/artifacts";
enum Mismatch {
    Label,
    Mount,
    Engine,
}

fn contract(snapshot: &GateSnapshot) -> Contract {
    contract_with_bazel(snapshot, std::path::Path::new(BAZEL_PATH))
}
fn contract_with_bazel(snapshot: &GateSnapshot, bazel: &std::path::Path) -> Contract {
    let execution = snapshot.execution().unwrap();
    let arguments =
        linux_gate_command::arguments(LINUX_AMD64_PROFILE, &execution, bazel, USER).unwrap();
    Contract::new(
        LINUX_AMD64_PROFILE,
        &execution,
        bazel,
        CONTAINER_BUILD_ROLE,
        &arguments,
    )
    .unwrap()
}
fn inspection(contract: &Contract) -> Inspection {
    Inspection {
        id: CONTAINER_ID.into(),
        name: format!("/{}", contract.command.name),
        config: Configuration {
            image: contract.image.clone(),
            cmd: contract.script.clone(),
            labels: std::collections::BTreeMap::from([(
                CONTAINER_OWNER_LABEL.into(),
                contract.command.owner.clone(),
            )]),
        },
        mounts: contract.mounts.clone(),
    }
}
fn remove_preserved(path: std::path::PathBuf) {
    yolo_release_filesystem::restore_owner_write(&path).unwrap();
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn completed_auto_removed_container_confirms_snapshot_release() {
    let snapshot = GateSnapshot::fixture().unwrap();
    let root = snapshot.root_path();
    let engine = TestEngine::default();
    Owner::prepare(&snapshot, &engine, contract(&snapshot), None)
        .unwrap()
        .finish(Ok(()))
        .unwrap();
    assert!(engine.removals.borrow().is_empty());
    drop(snapshot);
    assert!(!root.exists());
}

#[test]
fn cli_timeout_with_exact_cid_or_lost_cid_ack_settles_only_the_owned_container() {
    for cid_present in [false, true] {
        let snapshot = GateSnapshot::fixture().unwrap();
        let root = snapshot.root_path();
        let engine = TestEngine::default();
        let expected = contract(&snapshot);
        let observed = inspection(&expected);
        let cid = expected.command.cid_file.clone();
        let owner = Owner::prepare(&snapshot, &engine, expected, None).unwrap();
        *engine.current.borrow_mut() = Some(observed);
        if cid_present {
            std::fs::write(cid, CONTAINER_ID).unwrap();
        }
        assert!(owner.finish(Err(anyhow::anyhow!(CLIENT_FAILURE))).is_err());
        assert_eq!(*engine.removals.borrow(), [CONTAINER_ID]);
        drop(snapshot);
        assert!(!root.exists());
    }
}

#[test]
fn unacknowledged_absent_create_and_daemon_failures_preserve_mounted_snapshot() {
    for daemon_failure in [false, true] {
        let snapshot = GateSnapshot::fixture().unwrap();
        let root = snapshot.root_path();
        let engine = TestEngine::default();
        let owner = Owner::prepare(&snapshot, &engine, contract(&snapshot), None).unwrap();
        engine.fail_lookup.set(daemon_failure);
        assert!(owner.finish(Err(anyhow::anyhow!(CLIENT_FAILURE))).is_err());
        assert!(engine.removals.borrow().is_empty());
        drop(snapshot);
        assert!(root.is_dir());
        remove_preserved(root);
    }
}

#[test]
fn mismatched_identity_mounts_or_engine_never_authorize_removal() {
    for mismatch in [Mismatch::Label, Mismatch::Mount, Mismatch::Engine] {
        let snapshot = GateSnapshot::fixture().unwrap();
        let root = snapshot.root_path();
        let engine = TestEngine::default();
        let expected = contract(&snapshot);
        let mut observed = inspection(&expected);
        let owner = Owner::prepare(&snapshot, &engine, expected, None).unwrap();
        match mismatch {
            Mismatch::Label => {
                observed
                    .config
                    .labels
                    .insert(CONTAINER_OWNER_LABEL.into(), FOREIGN_OWNER.into());
            }
            Mismatch::Mount => observed.mounts.last_mut().unwrap().source = FOREIGN_MOUNT.into(),
            Mismatch::Engine => engine.replaced.set(true),
        }
        *engine.current.borrow_mut() = Some(observed);
        assert!(owner.finish(Err(anyhow::anyhow!(CLIENT_FAILURE))).is_err());
        assert!(engine.removals.borrow().is_empty());
        drop(snapshot);
        assert!(root.is_dir());
        remove_preserved(root);
    }
}

#[test]
fn cancellation_settles_owned_container_but_failed_cleanup_retains_evidence() {
    for failed in [false, true] {
        let snapshot = GateSnapshot::fixture().unwrap();
        let root = snapshot.root_path();
        let engine = TestEngine::default();
        let expected = contract(&snapshot);
        let observed = inspection(&expected);
        let owner = Owner::prepare(&snapshot, &engine, expected, None).unwrap();
        *engine.current.borrow_mut() = Some(observed);
        engine.fail_remove.set(failed);
        drop(owner);
        assert_eq!(*engine.removals.borrow(), [CONTAINER_ID]);
        drop(snapshot);
        assert_eq!(root.exists(), failed);
        if failed {
            remove_preserved(root);
        }
    }
}

#[test]
fn preexisting_exact_name_is_a_conflict_not_a_cleanup_target() {
    let snapshot = GateSnapshot::fixture().unwrap();
    let engine = TestEngine::default();
    let expected = contract(&snapshot);
    *engine.current.borrow_mut() = Some(inspection(&expected));
    assert!(Owner::prepare(&snapshot, &engine, expected, None).is_err());
    assert!(engine.removals.borrow().is_empty());
}

#[test]
fn complete_create_ack_and_exact_absence_release_but_malformed_cid_never_does() {
    for acknowledged in [false, true] {
        let snapshot = GateSnapshot::fixture().unwrap();
        let root = snapshot.root_path();
        let engine = TestEngine::default();
        let expected = contract(&snapshot);
        let cid = expected.command.cid_file.clone();
        let owner = Owner::prepare(&snapshot, &engine, expected, None).unwrap();
        std::fs::write(
            cid,
            if acknowledged {
                CONTAINER_ID
            } else {
                FOREIGN_OWNER
            },
        )
        .unwrap();
        assert!(owner.finish(Err(anyhow::anyhow!(CLIENT_FAILURE))).is_err());
        assert!(engine.removals.borrow().is_empty());
        drop(snapshot);
        assert_eq!(root.exists(), !acknowledged);
        if !acknowledged {
            remove_preserved(root);
        }
    }
}
