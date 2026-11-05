use super::{fixture::*, *};
use std::{
    fs,
    os::unix::{
        fs::{MetadataExt, PermissionsExt},
        net::UnixStream,
    },
};

#[test]
fn inactive_known_endpoints_are_quarantined_intact() {
    let fixture = Fixture::new();
    drop(fixture.listener(HOST));
    drop(fixture.listener(HUB));
    let identities: Vec<_> = [HOST, HUB]
        .map(|name| {
            fs::metadata(fixture.location.directory.join(name))
                .unwrap()
                .ino()
        })
        .into();
    fixture.recover().unwrap();
    assert!(!fixture.location.directory.exists());
    let paths = fixture.quarantines();
    assert_eq!(paths.len(), 1);
    for (name, inode) in [HOST, HUB].into_iter().zip(identities) {
        let metadata = fs::symlink_metadata(paths[0].join(name)).unwrap();
        assert_eq!(metadata.ino(), inode);
        assert_eq!(
            metadata.nlink(),
            crate::paths::constants::SOCKET_ENDPOINT_LINK_COUNT
        );
    }
    use crate::maintenance::{MaintenanceScanner, ScanChunk};
    let mut scanner = super::super::SocketNamespaceScanner::from_root(&fixture.root, true).unwrap();
    let mut completed = false;
    for _ in 0..crate::paths::constants::MAX_PRIVATE_DIRECTORY_ENTRIES {
        if scanner.scan_chunk().unwrap() == ScanChunk::PassComplete {
            completed = true;
            break;
        }
    }
    assert!(completed);
    assert!(paths[0].join(HOST).exists());
    assert!(paths[0].join(HUB).exists());
}

#[test]
fn live_endpoint_refuses_recovery_and_remains_connectable() {
    let fixture = Fixture::new();
    let listener = fixture.listener(HOST);
    assert!(fixture.recover().is_err());
    assert!(fixture.quarantines().is_empty());
    assert!(UnixStream::connect(fixture.location.directory.join(HOST)).is_ok());
    drop(listener);
}

#[test]
fn unknown_files_and_stage_publications_are_preserved_without_probing_or_moving() {
    for name in [UNKNOWN, STAGE] {
        let fixture = Fixture::new();
        drop(fixture.listener(HOST));
        let path = fixture.location.directory.join(name);
        fs::write(&path, CONTENT).unwrap();
        assert!(fixture.recover().is_err());
        assert_eq!(fs::read(path).unwrap(), CONTENT);
        assert!(fixture.quarantines().is_empty());
        assert!(fixture.location.directory.join(HOST).exists());
    }
}

#[test]
fn endpoint_mode_symlinks_and_extra_links_fail_closed() {
    for kind in 0..3 {
        let fixture = Fixture::new();
        let path = fixture.location.directory.join(HOST);
        if kind == 0 {
            drop(fixture.listener(HOST));
            fs::set_permissions(&path, fs::Permissions::from_mode(WRONG_MODE)).unwrap();
        } else if kind == 1 {
            let outside = fixture.root.path().join(UNKNOWN);
            fs::write(&outside, CONTENT).unwrap();
            std::os::unix::fs::symlink(outside, &path).unwrap();
        } else {
            drop(fixture.listener(HOST));
            fs::hard_link(&path, fixture.root.path().join(UNKNOWN)).unwrap();
        }
        assert!(fixture.recover().is_err());
        assert!(fs::symlink_metadata(path).is_ok());
        assert!(fixture.quarantines().is_empty());
    }
}

#[test]
fn held_stage_lifecycle_or_registration_failure_never_quarantines() {
    let fixture = Fixture::new();
    drop(fixture.listener(HOST));
    let target = socket_stage::Target::new(fixture.directory.path(), c"host.sock").unwrap();
    let stage = PrivateFileLock::try_acquire_in(
        fixture.directory.try_clone().unwrap(),
        &target.lifecycle_path(),
        SOCKET_STAGE_LIFECYCLE_MAX_BYTES,
    )
    .unwrap()
    .unwrap();
    assert!(fixture.recover().is_err());
    drop(stage);
    let mut lifecycle = SocketNamespaceLifecycleLock::acquire(&fixture.root).unwrap();
    assert!(
        recover(
            &fixture.root,
            &fixture.location,
            &fixture.directory,
            &mut lifecycle,
            || anyhow::bail!(CHANGED)
        )
        .is_err()
    );
    assert!(fixture.quarantines().is_empty());
}

#[test]
fn mutation_after_probe_prevents_rename_and_preserves_the_replacement() {
    let fixture = Fixture::new();
    drop(fixture.listener(HOST));
    let mut lifecycle = SocketNamespaceLifecycleLock::acquire(&fixture.root).unwrap();
    assert!(
        recover_with(
            &fixture.root,
            &fixture.location,
            &fixture.directory,
            &mut lifecycle,
            || Ok(()),
            || {
                fs::write(fixture.location.directory.join(UNKNOWN), CONTENT)?;
                Ok(())
            }
        )
        .is_err()
    );
    assert!(fixture.quarantines().is_empty());
    assert_eq!(
        fs::read(fixture.location.directory.join(UNKNOWN)).unwrap(),
        CONTENT
    );
    assert!(fixture.location.directory.join(HOST).exists());
}

#[test]
fn unknown_ownership_files_are_never_rewritten_or_recovered() {
    let fixture = Fixture::new();
    let marker = fixture.location.directory.join(UNKNOWN);
    fs::write(&marker, CONTENT).unwrap();
    assert!(fixture.recover().is_err());
    assert_eq!(fs::read(marker).unwrap(), CONTENT);
    assert!(fixture.quarantines().is_empty());
}

#[test]
fn lost_registration_authority_after_probing_prevents_quarantine() {
    let fixture = Fixture::new();
    drop(fixture.listener(HOST));
    let valid = std::cell::Cell::new(true);
    let mut lifecycle = SocketNamespaceLifecycleLock::acquire(&fixture.root).unwrap();
    assert!(
        recover_with(
            &fixture.root,
            &fixture.location,
            &fixture.directory,
            &mut lifecycle,
            || {
                ensure!(valid.get(), CHANGED);
                Ok(())
            },
            || {
                valid.set(false);
                Ok(())
            }
        )
        .is_err()
    );
    assert!(fixture.location.directory.join(HOST).exists());
    assert!(fixture.quarantines().is_empty());
}
