use super::probe_recovery_tests::{ProbeFixture, create_private_socket};
use super::*;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};

const UNSAFE_SOCKET_MODE: u32 = 0o700;

#[test]
fn unsafe_quarantine_shapes_are_preserved_table_driven() {
    for shape in [
        UnsafeShape::Symlink,
        UnsafeShape::Directory,
        UnsafeShape::WrongModeSocket,
    ] {
        let fixture = ProbeFixture::new();
        fs::remove_file(&fixture.probe).unwrap();
        match shape {
            UnsafeShape::Symlink => symlink(&fixture.endpoint, &fixture.quarantine).unwrap(),
            UnsafeShape::Directory => fs::create_dir(&fixture.quarantine).unwrap(),
            UnsafeShape::WrongModeSocket => {
                create_private_socket(&fixture.quarantine);
                fs::set_permissions(
                    &fixture.quarantine,
                    fs::Permissions::from_mode(UNSAFE_SOCKET_MODE),
                )
                .unwrap();
            }
        }

        assert_unproven(&fixture);
        match shape {
            UnsafeShape::Symlink => assert!(
                fs::symlink_metadata(&fixture.quarantine)
                    .unwrap()
                    .file_type()
                    .is_symlink()
            ),
            UnsafeShape::Directory => assert!(fixture.quarantine.is_dir()),
            UnsafeShape::WrongModeSocket => assert_eq!(
                fs::symlink_metadata(&fixture.quarantine)
                    .unwrap()
                    .permissions()
                    .mode()
                    & PERMISSION_MODE_MASK,
                UNSAFE_SOCKET_MODE
            ),
        }
    }
}

#[test]
fn quarantine_created_at_the_commit_fence_blocks_probe_migration_without_deletion() {
    let fixture = ProbeFixture::new();
    let probe_inode = inode(&fixture.probe);
    let quarantine = fixture.quarantine.clone();

    let cleanup = socket_stage::cleanup_key_with_test_hooks(
        &fixture.directory,
        fixture.target.key(),
        fixture.directory.path(),
        || {
            create_private_socket(&quarantine);
            Ok(())
        },
        || Ok(()),
    )
    .unwrap();

    assert!(matches!(cleanup, socket_stage::Cleanup::Unproven));
    assert_eq!(inode(&fixture.probe), probe_inode);
    assert!(fixture.quarantine.exists());
    assert!(fixture.lease.exists());
}

#[test]
fn quarantine_created_after_the_absence_proof_fails_the_exclusive_rename() {
    let fixture = ProbeFixture::new();
    let quarantine = fixture.quarantine.clone();
    let _hook = socket_stage::install_probe_cleanup_rename_hook(move || {
        create_private_socket(&quarantine);
        Ok(())
    });

    let cleanup = socket_stage::cleanup_key(&fixture.directory, fixture.target.key()).unwrap();

    assert!(matches!(cleanup, socket_stage::Cleanup::Unproven));
    assert!(fixture.probe.exists());
    assert!(fixture.quarantine.exists());
    assert!(fixture.lease.exists());
}

#[test]
fn single_link_quarantine_replacement_before_unlink_is_preserved() {
    let fixture = ProbeFixture::new();
    fs::rename(&fixture.probe, &fixture.quarantine).unwrap();
    fs::remove_file(&fixture.endpoint).unwrap();
    fixture.directory.sync().unwrap();
    let original_inode = inode(&fixture.quarantine);
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
    assert_ne!(inode(&fixture.quarantine), original_inode);
    assert!(fixture.lease.exists());
}

#[derive(Clone, Copy)]
enum UnsafeShape {
    Symlink,
    Directory,
    WrongModeSocket,
}

fn assert_unproven(fixture: &ProbeFixture) {
    assert!(matches!(
        socket_stage::cleanup_key(&fixture.directory, fixture.target.key()).unwrap(),
        socket_stage::Cleanup::Unproven
    ));
    assert!(fixture.quarantine.exists());
    assert!(fixture.lease.exists());
}

fn inode(path: &Path) -> u64 {
    fs::symlink_metadata(path).unwrap().ino()
}
