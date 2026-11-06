use super::probe_recovery_tests::{ProbeFixture, create_private_socket};
use super::*;
use std::fs;
use std::os::unix::fs::MetadataExt;

const SAVED_ENDPOINT_NAME: &str = "saved-proof-endpoint";

#[test]
fn double_link_source_reappearance_blocks_the_first_unlink_proof() {
    let fixture = ProbeFixture::new();
    let probe = fixture.probe.clone();

    let cleanup = cleanup_with_after_quarantine(&fixture, || {
        create_private_socket(&probe);
        Ok(())
    });

    assert!(matches!(cleanup, socket_stage::Cleanup::Unproven));
    assert!(fixture.probe.exists());
    assert!(fixture.quarantine.exists());
    assert!(fixture.lease.exists());
}

#[test]
fn single_link_replacement_before_quarantine_fails_alias_proof() {
    let fixture = ProbeFixture::new();
    fs::remove_file(&fixture.endpoint).unwrap();
    fixture.directory.sync().unwrap();
    let original_inode = inode(&fixture.probe);
    let probe = fixture.probe.clone();

    let cleanup = socket_stage::cleanup_key_with_all_test_hooks(
        &fixture.directory,
        fixture.target.key(),
        fixture.directory.path(),
        || {
            fs::remove_file(&probe)?;
            create_private_socket(&probe);
            Ok(())
        },
        || Ok(()),
        || Ok(()),
        || Ok(()),
    )
    .unwrap();

    assert!(matches!(cleanup, socket_stage::Cleanup::Unproven));
    assert_ne!(inode(&fixture.probe), original_inode);
    assert!(!fixture.quarantine.exists());
    assert!(fixture.lease.exists());
}

#[test]
fn single_link_source_reappearance_blocks_the_first_unlink_proof() {
    let fixture = ProbeFixture::new();
    fs::remove_file(&fixture.endpoint).unwrap();
    fixture.directory.sync().unwrap();
    let probe = fixture.probe.clone();

    let cleanup = cleanup_with_after_quarantine(&fixture, || {
        create_private_socket(&probe);
        Ok(())
    });

    assert!(matches!(cleanup, socket_stage::Cleanup::Unproven));
    assert!(fixture.probe.exists());
    assert!(fixture.quarantine.exists());
    assert!(fixture.lease.exists());
}

#[test]
fn occupied_probe_name_prevents_unproven_quarantine_restore() {
    let fixture = ProbeFixture::new();
    let original_inode = inode(&fixture.endpoint);
    let endpoint = fixture.endpoint.clone();
    let probe = fixture.probe.clone();
    let saved = fixture.directory.path().join(SAVED_ENDPOINT_NAME);

    let cleanup = cleanup_with_after_quarantine(&fixture, || {
        fs::rename(&endpoint, &saved)?;
        create_private_socket(&endpoint);
        create_private_socket(&probe);
        Ok(())
    });

    assert!(matches!(cleanup, socket_stage::Cleanup::Unproven));
    assert_eq!(inode(&saved), original_inode);
    assert_eq!(inode(&fixture.quarantine), original_inode);
    assert_ne!(inode(&fixture.probe), original_inode);
    assert!(fixture.lease.exists());
}

fn cleanup_with_after_quarantine<F>(
    fixture: &ProbeFixture,
    after_quarantine: F,
) -> socket_stage::Cleanup
where
    F: FnMut() -> anyhow::Result<()>,
{
    socket_stage::cleanup_key_with_all_test_hooks(
        &fixture.directory,
        fixture.target.key(),
        fixture.directory.path(),
        || Ok(()),
        after_quarantine,
        || Ok(()),
        || Ok(()),
    )
    .unwrap()
}

fn inode(path: &Path) -> u64 {
    fs::symlink_metadata(path).unwrap().ino()
}
