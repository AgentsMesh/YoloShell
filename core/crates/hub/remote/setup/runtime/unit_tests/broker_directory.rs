use crate::broker_directory::BrokerDirectory;
use std::os::unix::fs::PermissionsExt;
use yolo_hub_ssh_profile_protocol::SshBrokerProfile;
use yolo_paths::directory::PrivateDirectory;

const BROKER: &str = "broker";
const FOREIGN: &str = "foreign";
const RETIRED: &str = "retired";
const PRIVATE_MODE: u32 = 0o700;
const UNSAFE_MODE: u32 = 0o755;
const MODE_MASK: u32 = 0o777;

fn registration(directory: &PrivateDirectory) -> SshBrokerProfile {
    let request = yolo_hub_remote_setup_protocol_test_support::fixture::request();
    SshBrokerProfile {
        profile: request.profile().unwrap(),
        broker_state: directory.path().to_path_buf(),
        config_revision: request.request.fields().profile.config_revision.clone(),
        launch: None,
    }
}

#[test]
fn durable_reopen_mints_live_fence_and_replacement_revokes_it() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().canonicalize().unwrap().join(BROKER);
    let directory = PrivateDirectory::ensure(&path).unwrap();
    let profile = registration(&directory);
    let identity = directory.durable_identity().unwrap();
    let retained = BrokerDirectory::open(&profile, Some(identity)).unwrap();
    assert_eq!(retained.identity(), identity);
    retained.verify().unwrap();
    drop(retained);
    let reopened = BrokerDirectory::open(&profile, Some(identity)).unwrap();
    reopened.verify().unwrap();
    std::fs::rename(&path, root.path().join(RETIRED)).unwrap();
    PrivateDirectory::ensure(&path).unwrap();
    assert!(reopened.verify().is_err());
    assert!(BrokerDirectory::open(&profile, Some(identity)).is_err());
}

#[test]
fn foreign_durable_proof_is_rejected_before_broker_admission() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    let directory = PrivateDirectory::ensure(&root.join(BROKER)).unwrap();
    let foreign = PrivateDirectory::ensure(&root.join(FOREIGN)).unwrap();
    assert!(
        BrokerDirectory::open(
            &registration(&directory),
            Some(foreign.durable_identity().unwrap()),
        )
        .is_err()
    );
}

#[test]
fn missing_or_symlinked_broker_directory_never_acquires_authority_or_repairs_state() {
    for symlinked in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let root = root.path().canonicalize().unwrap();
        let path = root.join(BROKER);
        let directory = PrivateDirectory::ensure(&path).unwrap();
        let profile = registration(&directory);
        let identity = directory.durable_identity().unwrap();
        let held = root.join(RETIRED);
        std::fs::rename(&path, &held).unwrap();
        if symlinked {
            std::os::unix::fs::symlink(&held, &path).unwrap();
        }
        for expected in [None, Some(identity)] {
            assert!(BrokerDirectory::open(&profile, expected).is_err());
        }
        assert_eq!(std::fs::symlink_metadata(&path).is_ok(), symlinked);
        if symlinked {
            assert_eq!(std::fs::read_link(&path).unwrap(), held);
        }
        assert_eq!(std::fs::read_dir(&held).unwrap().count(), usize::default());
    }
}

#[test]
fn unsafe_permissions_revoke_retained_broker_and_are_not_normalized_by_open() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().canonicalize().unwrap().join(BROKER);
    let directory = PrivateDirectory::ensure(&path).unwrap();
    let profile = registration(&directory);
    let identity = directory.durable_identity().unwrap();
    let retained = BrokerDirectory::open(&profile, Some(identity)).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(UNSAFE_MODE)).unwrap();
    assert!(retained.verify().is_err());
    for expected in [None, Some(identity)] {
        assert!(BrokerDirectory::open(&profile, expected).is_err());
    }
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & MODE_MASK,
        UNSAFE_MODE
    );
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(PRIVATE_MODE)).unwrap();
    let reopened = BrokerDirectory::open(&profile, Some(identity)).unwrap();
    assert_eq!(reopened.identity(), identity);
    reopened.verify().unwrap();
}
