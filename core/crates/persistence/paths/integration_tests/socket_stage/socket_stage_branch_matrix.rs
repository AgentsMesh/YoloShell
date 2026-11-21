use super::super::{
    cleanup_io,
    cleanup_lease::{self, LeaseState},
    lease, probe_target, record,
};
use super::*;
use crate::paths::constants::{SIZE_LIMIT_SENTINEL_BYTES, SOCKET_STAGE_LEASE_MAX_BYTES};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;

const ALTERNATE_ENDPOINT_NAME: &str = "alternate.sock";
const UNSAFE_STAGE_BYTES: &[u8] = b"not-a-socket";
const INVALID_LEASE_BYTES: &[u8] = b"{";
const IDENTITY_OFFSET: libc::ino_t = 1;
const LINK_COUNT_OFFSET: u64 = 1;
const USER_ID_OFFSET: u32 = 1;

#[tokio::test(flavor = "current_thread")]
async fn async_lease_acquires_without_contention() {
    let (_temporary, directory, target) = super::fixture();
    let lease = Lease::acquire(&directory, &target).await.unwrap();
    drop(lease);
}

#[test]
fn cleanup_state_classifies_missing_and_unproven_lease_records() {
    let (_temporary, directory, target) = super::fixture();
    assert!(matches!(
        cleanup_lease::try_cleanup(&directory, target.key(), directory.path()).unwrap(),
        LeaseState::Busy
    ));

    drop(Lease::acquire_immediate_for_test(&directory, &target).unwrap());
    let lease_path = target.lease().unwrap().1;
    let oversized = fs::OpenOptions::new()
        .write(true)
        .open(&lease_path)
        .unwrap();
    oversized
        .set_len(SOCKET_STAGE_LEASE_MAX_BYTES + SIZE_LIMIT_SENTINEL_BYTES)
        .unwrap();
    assert!(matches!(
        cleanup_lease::try_cleanup(&directory, target.key(), directory.path()).unwrap(),
        LeaseState::Unproven
    ));

    fs::write(&lease_path, INVALID_LEASE_BYTES).unwrap();
    assert!(matches!(
        cleanup_lease::try_cleanup(&directory, target.key(), directory.path()).unwrap(),
        LeaseState::Unproven
    ));

    let alternate_name = CString::new(ALTERNATE_ENDPOINT_NAME).unwrap();
    let alternate = Target::new(directory.path(), &alternate_name).unwrap();
    assert_ne!(alternate.key(), target.key());
    fs::write(&lease_path, record::encode(&alternate).unwrap()).unwrap();
    assert!(matches!(
        cleanup_lease::try_cleanup(&directory, target.key(), directory.path()).unwrap(),
        LeaseState::Unproven
    ));

    let mut wrong_user = record::decode(&record::encode(&target).unwrap()).unwrap();
    wrong_user.effective_user_id = wrong_user.effective_user_id.wrapping_add(USER_ID_OFFSET);
    fs::write(&lease_path, serde_json::to_vec(&wrong_user).unwrap()).unwrap();
    assert!(matches!(
        cleanup_lease::try_cleanup(&directory, target.key(), directory.path()).unwrap(),
        LeaseState::Unproven
    ));

    fs::write(&lease_path, record::encode(&target).unwrap()).unwrap();
    let LeaseState::Acquired(cleanup) =
        cleanup_lease::try_cleanup(&directory, target.key(), directory.path()).unwrap()
    else {
        panic!("matching lease record was not acquired");
    };
    assert_eq!(cleanup.target().key(), target.key());
    assert!(cleanup.remove().unwrap());
    assert!(!lease_path.exists());
}

#[test]
fn lifecycle_lock_and_stage_residue_fail_closed() {
    let (_temporary, directory, target) = super::fixture();
    let lifecycle = lease::acquire_lifecycle(&directory, &target.lifecycle_path())
        .unwrap()
        .unwrap();
    assert!(matches!(
        cleanup_lease::try_cleanup(&directory, target.key(), directory.path()).unwrap(),
        LeaseState::Busy
    ));
    assert!(Lease::acquire_immediate_for_test(&directory, &target).is_err());
    drop(lifecycle);

    let (_, bind_stage) = target.stage(Kind::Bind).unwrap();
    let listener = private_socket(&bind_stage);
    assert!(Lease::acquire_immediate_for_test(&directory, &target).is_err());
    assert!(bind_stage.exists());
    drop(listener);
}

#[test]
fn stage_inspection_propagates_non_absence_errors() {
    const BLOCKING_ENTRY_NAME: &str = "stage-inspection-blocker";
    const BLOCKING_ENTRY_BYTES: &[u8] = b"preserve";
    const BLOCKED_CHILD_NAME: &str = "child";

    let (_temporary, directory, _target) = super::fixture();
    let blocker = directory.path().join(BLOCKING_ENTRY_NAME);
    fs::write(&blocker, BLOCKING_ENTRY_BYTES).unwrap();
    let blocked = CString::new(format!("{BLOCKING_ENTRY_NAME}/{BLOCKED_CHILD_NAME}")).unwrap();
    assert!(lease::stage_exists(&directory, &blocked).is_err());
    assert_eq!(fs::read(blocker).unwrap(), BLOCKING_ENTRY_BYTES);
}

#[test]
fn probe_target_checks_explicit_and_scanned_endpoint_evidence() {
    let (_temporary, directory, target) = super::fixture();
    let endpoint_name = CString::new(HOST_SOCKET_FILE_NAME).unwrap();
    let endpoint = directory.path().join(HOST_SOCKET_FILE_NAME);
    let listener = private_socket(&endpoint);
    let state = match cleanup_io::inspect(&directory, &endpoint_name).unwrap() {
        cleanup_io::Inspection::Socket(state) => state,
        _ => panic!("private endpoint was not classified as a socket"),
    };

    assert!(
        probe_target::has_links(
            &directory,
            &target,
            Some(&endpoint_name),
            state.identity,
            state.links,
        )
        .unwrap()
    );
    assert!(
        probe_target::has_links(&directory, &target, None, state.identity, state.links).unwrap()
    );
    assert!(
        !probe_target::has_links(
            &directory,
            &target,
            Some(&endpoint_name),
            cleanup_io::Identity {
                inode: state.identity.inode + IDENTITY_OFFSET,
                ..state.identity
            },
            state.links,
        )
        .unwrap()
    );
    assert!(
        !probe_target::has_links(
            &directory,
            &target,
            Some(&endpoint_name),
            state.identity,
            state.links + LINK_COUNT_OFFSET,
        )
        .unwrap()
    );
    let alternate = CString::new(ALTERNATE_ENDPOINT_NAME).unwrap();
    assert!(
        !probe_target::has_links(
            &directory,
            &target,
            Some(&alternate),
            state.identity,
            state.links,
        )
        .unwrap()
    );

    assert!(probe_target::probe_slot_missing(&directory, &target).unwrap());
    let (_, probe) = target.stage(Kind::Probe).unwrap();
    fs::write(&probe, UNSAFE_STAGE_BYTES).unwrap();
    assert!(!probe_target::probe_slot_missing(&directory, &target).unwrap());
    drop(listener);
}

fn private_socket(path: &Path) -> UnixListener {
    let listener = UnixListener::bind(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    listener
}
