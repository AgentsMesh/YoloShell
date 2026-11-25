use super::probe_recovery_tests::{ProbeFixture, create_private_socket};
use super::*;
use std::fs;
use std::os::unix::fs::MetadataExt;

const EXTRA_ALIAS_NAME: &str = "probe-extra-alias";
const SAVED_ENDPOINT_NAME: &str = "saved-probe-endpoint";

#[test]
fn quarantine_with_an_unexpected_link_count_is_preserved() {
    let fixture = ProbeFixture::new();
    fs::rename(&fixture.probe, &fixture.quarantine).unwrap();
    let extra = fixture.directory.path().join(EXTRA_ALIAS_NAME);
    fs::hard_link(&fixture.quarantine, &extra).unwrap();
    fixture.directory.sync().unwrap();

    let cleanup = socket_stage::cleanup_key(&fixture.directory, fixture.target.key()).unwrap();

    assert!(matches!(cleanup, socket_stage::Cleanup::Unproven));
    assert!(fixture.quarantine.exists());
    assert!(extra.exists());
    assert!(fixture.lease.exists());
}

#[test]
fn unrelated_double_link_probe_fails_before_quarantine() {
    let fixture = ProbeFixture::new();
    fs::remove_file(&fixture.probe).unwrap();
    create_private_socket(&fixture.probe);
    let extra = fixture.directory.path().join(EXTRA_ALIAS_NAME);
    fs::hard_link(&fixture.probe, &extra).unwrap();
    fixture.directory.sync().unwrap();

    let cleanup = socket_stage::cleanup_key(&fixture.directory, fixture.target.key()).unwrap();

    assert!(matches!(cleanup, socket_stage::Cleanup::Unproven));
    assert!(fixture.probe.exists());
    assert!(!fixture.quarantine.exists());
    assert!(fixture.lease.exists());
}

#[test]
fn endpoint_replacement_after_quarantine_restores_the_probe_name() {
    let fixture = ProbeFixture::new();
    let original_inode = inode(&fixture.endpoint);
    let saved = fixture.directory.path().join(SAVED_ENDPOINT_NAME);
    let endpoint = fixture.endpoint.clone();

    let cleanup = socket_stage::cleanup_key_with_all_test_hooks(
        &fixture.directory,
        fixture.target.key(),
        fixture.directory.path(),
        || Ok(()),
        || {
            fs::rename(&endpoint, &saved)?;
            create_private_socket(&endpoint);
            Ok(())
        },
        || Ok(()),
        || Ok(()),
    )
    .unwrap();

    assert!(matches!(cleanup, socket_stage::Cleanup::Unproven));
    assert_eq!(inode(&fixture.probe), original_inode);
    assert_eq!(inode(&saved), original_inode);
    assert_ne!(inode(&fixture.endpoint), original_inode);
    assert!(!fixture.quarantine.exists());
}

#[test]
fn post_unlink_replacements_fail_each_commit_proof() {
    for replacement in [
        PostUnlinkReplacement::Quarantine,
        PostUnlinkReplacement::Probe,
        PostUnlinkReplacement::Endpoint,
    ] {
        assert_post_unlink_replacement(replacement);
    }
}

#[test]
fn single_link_probe_replacement_after_unlink_preserves_the_lease() {
    let fixture = ProbeFixture::new();
    fs::remove_file(&fixture.endpoint).unwrap();
    fixture.directory.sync().unwrap();
    let probe = fixture.probe.clone();

    let cleanup = socket_stage::cleanup_key_with_all_test_hooks(
        &fixture.directory,
        fixture.target.key(),
        fixture.directory.path(),
        || Ok(()),
        || Ok(()),
        || Ok(()),
        || {
            create_private_socket(&probe);
            Ok(())
        },
    )
    .unwrap();

    assert!(matches!(cleanup, socket_stage::Cleanup::Unproven));
    assert!(fixture.probe.exists());
    assert!(fixture.lease.exists());
}

#[derive(Clone, Copy)]
enum PostUnlinkReplacement {
    Quarantine,
    Probe,
    Endpoint,
}

fn assert_post_unlink_replacement(replacement: PostUnlinkReplacement) {
    let fixture = ProbeFixture::new();
    let quarantine = fixture.quarantine.clone();
    let probe = fixture.probe.clone();
    let endpoint = fixture.endpoint.clone();
    let saved = fixture.directory.path().join(SAVED_ENDPOINT_NAME);
    let cleanup = socket_stage::cleanup_key_with_all_test_hooks(
        &fixture.directory,
        fixture.target.key(),
        fixture.directory.path(),
        || Ok(()),
        || Ok(()),
        || Ok(()),
        || {
            match replacement {
                PostUnlinkReplacement::Quarantine => {
                    create_private_socket(&quarantine);
                }
                PostUnlinkReplacement::Probe => {
                    create_private_socket(&probe);
                }
                PostUnlinkReplacement::Endpoint => {
                    fs::rename(&endpoint, &saved)?;
                    create_private_socket(&endpoint);
                }
            };
            Ok(())
        },
    )
    .unwrap();
    assert!(matches!(cleanup, socket_stage::Cleanup::Unproven));
    assert!(fixture.lease.exists());
}

fn inode(path: &Path) -> u64 {
    fs::symlink_metadata(path).unwrap().ino()
}
