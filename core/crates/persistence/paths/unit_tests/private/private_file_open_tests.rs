use super::*;
use std::io::{Read, Seek, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::sync::mpsc;
use std::time::Duration;

const CHANNEL_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_BYTES: u64 = 32;
const INJECTED_DIRECTORY_SYNC_FAILURE: &str = "injected private directory sync failure";
const REPLACED_NAME: &str = "replaced-lock";
const TARGET_NAME: &str = "writer-lock";
const SIMULATED_UNLINK_INTERRUPTION: &str = "simulated staged creation unlink interruption";
const UNEXPECTED_TEMPORARY_RETRY: &str = "single-attempt creation retried internally";

#[test]
fn exclusive_create_conflict_opens_the_winning_file() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join(TARGET_NAME);
    let entry = PrivateFileEntry::ensure(&target).unwrap();
    private_file_missing::set_before_create_hook({
        let target = target.clone();
        move || create_private(&target).unwrap()
    });

    let opened = entry.open_rw_or_create(MAX_BYTES).unwrap();

    entry
        .verify_current_unchanged(opened.identity, MAX_BYTES)
        .unwrap();
}

#[test]
fn exclusive_create_conflict_rejects_path_replacement() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join(TARGET_NAME);
    let replaced = root.path().join(REPLACED_NAME);
    let entry = PrivateFileEntry::ensure(&target).unwrap();
    private_file_missing::set_before_create_hook({
        let target = target.clone();
        move || create_private(&target).unwrap()
    });
    set_after_open_hook({
        let target = target.clone();
        move || {
            std::fs::rename(&target, &replaced).unwrap();
            create_private(&target).unwrap();
        }
    });

    assert!(entry.open_rw_or_create(MAX_BYTES).is_err());
}

#[test]
fn staged_creation_keeps_target_hidden_until_publish() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join(TARGET_NAME);
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let creator_target = target.clone();
    let creator = std::thread::spawn(move || {
        let entry = PrivateFileEntry::ensure(&creator_target).unwrap();
        let _hook = entry.install_after_stage_identity_hook(move || {
            started_tx.send(()).unwrap();
            release_rx.recv_timeout(CHANNEL_TIMEOUT).unwrap();
            Ok(())
        });
        entry.open_rw_or_create(MAX_BYTES).map(drop)
    });
    started_rx.recv_timeout(CHANNEL_TIMEOUT).unwrap();

    let contender_target = target.clone();
    let contender = std::thread::spawn(move || {
        PrivateFileEntry::ensure(&contender_target)
            .unwrap()
            .open_rw_or_create(MAX_BYTES)
    });
    assert!(!target.exists());
    release_tx.send(()).unwrap();
    creator.join().unwrap().unwrap();
    let opened = match contender.join().unwrap() {
        Ok(opened) => opened,
        Err(error) if io_kind(&error) == Some(std::io::ErrorKind::WouldBlock) => {
            PrivateFileEntry::ensure(&target)
                .unwrap()
                .open_rw_or_create(MAX_BYTES)
                .unwrap()
        }
        Err(error) => panic!("unexpected contender error: {error:#}"),
    };
    let mut opened = opened;
    opened.file.write_all(b"writable").unwrap();
    opened.file.sync_all().unwrap();
    opened.file.rewind().unwrap();
    let mut bytes = Vec::new();
    opened.file.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"writable");
    drop(opened);
    assert_only_target(root.path());
}

#[test]
fn published_directory_sync_failure_is_not_treated_as_a_creation_collision() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join(TARGET_NAME);
    let entry = PrivateFileEntry::ensure(&target).unwrap();
    let _hook = entry.install_after_stage_identity_hook(|| {
        crate::paths::directory::fail_next_sync();
        Ok(())
    });

    let error = entry.open_rw_or_create(MAX_BYTES).unwrap_err();

    assert!(target.exists());
    assert!(format!("{error:#}").contains(INJECTED_DIRECTORY_SYNC_FAILURE));
}

#[test]
fn try_acquire_does_not_retry_a_busy_creation_stage() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join(TARGET_NAME);
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let creator_target = target.clone();
    let creator = std::thread::spawn(move || {
        let entry = PrivateFileEntry::ensure(&creator_target).unwrap();
        let _hook = entry.install_after_stage_identity_hook(move || {
            started_tx.send(()).unwrap();
            release_rx.recv_timeout(CHANNEL_TIMEOUT).unwrap();
            Ok(())
        });
        entry.open_rw_or_create(MAX_BYTES).map(drop)
    });
    started_rx.recv_timeout(CHANNEL_TIMEOUT).unwrap();
    let entry = PrivateFileEntry::ensure(&target).unwrap();
    let _retry =
        entry.install_before_temporary_retry_hook(|| anyhow::bail!(UNEXPECTED_TEMPORARY_RETRY));

    assert!(
        crate::paths::private_file_lock::PrivateFileLock::try_acquire(&target, MAX_BYTES)
            .unwrap()
            .is_none()
    );

    release_tx.send(()).unwrap();
    creator.join().unwrap().unwrap();
}

#[test]
fn abandoned_unlink_isolation_is_recovered_before_recreating_target() {
    let root = tempfile::tempdir().unwrap();
    PrivateDirectory::ensure(root.path()).unwrap();
    let target = root.path().join(TARGET_NAME);
    create_private(&target).unwrap();
    let entry = PrivateFileEntry::open(&target).unwrap();
    let temporary = entry
        .temporary_single_attempt(
            crate::paths::constants::ATOMIC_FILE_PREFIX,
            crate::paths::constants::ATOMIC_FILE_SUFFIX,
        )
        .unwrap();
    let _interruption =
        entry.install_before_unlink_hook(|| anyhow::bail!(SIMULATED_UNLINK_INTERRUPTION));
    assert!(
        entry
            .unlink_named_identity(&temporary.name, temporary.identity, MAX_BYTES, || Ok(()),)
            .is_err()
    );
    drop(temporary);
    std::fs::remove_file(&target).unwrap();

    drop(entry.open_rw_or_create(MAX_BYTES).unwrap());

    assert_only_target(root.path());
}

fn assert_only_target(directory: &std::path::Path) {
    let names: Vec<_> = std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(names, [std::ffi::OsString::from(TARGET_NAME)]);
}

fn create_private(path: &std::path::Path) -> std::io::Result<()> {
    let file = std::fs::OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .mode(PRIVATE_FILE_MODE)
        .open(path)?;
    file.set_permissions(std::fs::Permissions::from_mode(PRIVATE_FILE_MODE))?;
    file.sync_all()
}
