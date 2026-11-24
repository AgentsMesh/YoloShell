use super::probe_recovery_tests::{ProbeFixture, create_private_socket};
use super::*;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};

const EXTRA_LINK_NAME: &str = "unexpected-probe-link";
const EXTRA_LINK_COUNT: u64 = SOCKET_PROBE_LINK_COUNT + EXPECTED_FILE_LINK_COUNT;
const BIND_LINK_NAME: &str = "unexpected-bind-link";

#[test]
fn extra_hardlink_preserves_probe_target_and_bookkeeping() {
    let fixture = ProbeFixture::new();
    let extra = fixture.directory.path().join(EXTRA_LINK_NAME);
    fs::hard_link(&fixture.endpoint, &extra).unwrap();

    assert_unproven(&fixture);

    assert_eq!(
        fs::symlink_metadata(&fixture.endpoint).unwrap().nlink(),
        EXTRA_LINK_COUNT
    );
    assert!(fixture.probe.exists());
    assert!(extra.exists());
}

#[test]
fn alias_replacement_before_quarantine_is_preserved() {
    let fixture = ProbeFixture::new();
    let endpoint_inode = inode(&fixture.endpoint);
    let probe = fixture.probe.clone();

    let cleanup = socket_stage::cleanup_key_with_test_hooks(
        &fixture.directory,
        fixture.target.key(),
        fixture.directory.path(),
        || {
            fs::remove_file(&probe)?;
            create_private_socket(&probe);
            Ok(())
        },
        || Ok(()),
    )
    .unwrap();

    assert!(matches!(cleanup, socket_stage::Cleanup::Unproven));
    assert_ne!(inode(&fixture.probe), endpoint_inode);
    assert!(fixture.lease.exists());
    assert!(!fixture.quarantine.exists());
}

#[test]
fn target_replacement_before_quarantine_unlink_is_preserved() {
    let fixture = ProbeFixture::new();
    let original_inode = inode(&fixture.endpoint);
    let endpoint = fixture.endpoint.clone();

    let cleanup = socket_stage::cleanup_key_with_test_hooks(
        &fixture.directory,
        fixture.target.key(),
        fixture.directory.path(),
        || Ok(()),
        || {
            fs::remove_file(&endpoint)?;
            create_private_socket(&endpoint);
            Ok(())
        },
    )
    .unwrap();

    assert!(matches!(cleanup, socket_stage::Cleanup::Unproven));
    assert_ne!(inode(&fixture.endpoint), original_inode);
    assert_eq!(inode(&fixture.quarantine), original_inode);
    assert!(fixture.lease.exists());
}

#[test]
fn quarantine_replacement_before_unlink_is_preserved() {
    let fixture = ProbeFixture::new();
    let endpoint_inode = inode(&fixture.endpoint);
    let quarantine = fixture.quarantine.clone();

    let cleanup = socket_stage::cleanup_key_with_test_hooks(
        &fixture.directory,
        fixture.target.key(),
        fixture.directory.path(),
        || Ok(()),
        || {
            fs::remove_file(&quarantine)?;
            create_private_socket(&quarantine);
            Ok(())
        },
    )
    .unwrap();

    assert!(matches!(cleanup, socket_stage::Cleanup::Unproven));
    assert_ne!(inode(&fixture.quarantine), endpoint_inode);
    assert_eq!(inode(&fixture.endpoint), endpoint_inode);
    assert!(fixture.lease.exists());
}

#[test]
fn source_replacement_after_quarantine_is_preserved_with_the_lease() {
    let fixture = ProbeFixture::new();
    let endpoint_inode = inode(&fixture.endpoint);
    let probe = fixture.probe.clone();

    let cleanup = socket_stage::cleanup_key_with_test_hooks(
        &fixture.directory,
        fixture.target.key(),
        fixture.directory.path(),
        || Ok(()),
        || {
            create_private_socket(&probe);
            Ok(())
        },
    )
    .unwrap();

    assert!(matches!(cleanup, socket_stage::Cleanup::Unproven));
    assert_ne!(inode(&fixture.probe), endpoint_inode);
    assert_eq!(inode(&fixture.quarantine), endpoint_inode);
    assert!(fixture.lease.exists());
}

#[test]
fn externally_linked_bind_stage_is_preserved_with_the_lease() {
    let fixture = ProbeFixture::new();
    fs::remove_file(&fixture.probe).unwrap();
    let bind = fixture.target.stage(socket_stage::Kind::Bind).unwrap().1;
    let extra = fixture.directory.path().join(BIND_LINK_NAME);
    let bind_inode = create_private_socket(&bind);
    fs::hard_link(&bind, &extra).unwrap();
    fixture.directory.sync().unwrap();

    assert_unproven(&fixture);

    assert_eq!(inode(&bind), bind_inode);
    assert_eq!(inode(&extra), bind_inode);
    assert_eq!(
        fs::symlink_metadata(&bind).unwrap().nlink(),
        SOCKET_PROBE_LINK_COUNT
    );
}

#[test]
fn unsafe_probe_shapes_are_preserved_without_following_or_removal() {
    let symlinked = ProbeFixture::new();
    fs::remove_file(&symlinked.probe).unwrap();
    symlink(&symlinked.endpoint, &symlinked.probe).unwrap();
    assert_unproven(&symlinked);
    assert!(
        fs::symlink_metadata(&symlinked.probe)
            .unwrap()
            .file_type()
            .is_symlink()
    );

    let directory = ProbeFixture::new();
    fs::remove_file(&directory.probe).unwrap();
    fs::create_dir(&directory.probe).unwrap();
    assert_unproven(&directory);
    assert!(directory.probe.is_dir());

    let wrong_mode = ProbeFixture::new();
    fs::set_permissions(
        &wrong_mode.probe,
        fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
    )
    .unwrap();
    assert_unproven(&wrong_mode);
    assert!(wrong_mode.probe.exists());
}

fn assert_unproven(fixture: &ProbeFixture) {
    let cleanup = socket_stage::cleanup_key(&fixture.directory, fixture.target.key()).unwrap();
    assert!(matches!(cleanup, socket_stage::Cleanup::Unproven));
    assert!(fixture.lease.exists());
    assert!(fixture.lifecycle.exists());
}

fn inode(path: &Path) -> u64 {
    fs::symlink_metadata(path).unwrap().ino()
}
