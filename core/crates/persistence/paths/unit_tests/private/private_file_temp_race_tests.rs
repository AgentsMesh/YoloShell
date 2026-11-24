use super::*;
use serde_json::json;
use std::io::Write;
use std::sync::mpsc;
use std::time::Duration;

const TARGET_NAME: &str = "race.json";
const TEMPORARY_PREFIX: &str = ".yolo-race-";
const TEMPORARY_SUFFIX: &str = ".tmp";
const TRUSTED_BYTES: &[u8] = b"{\"state\":\"trusted\"}";
const RACE_TIMEOUT: Duration = Duration::from_secs(10);
const ORIGINAL_STATE: &str = "original";
const REPLACEMENT_STATE: &str = "replacement";

#[test]
fn active_open_before_lock_never_truncates_the_published_target() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join(TARGET_NAME);
    let waiter = spawn_stage_waiter(target.clone());
    waiter.wait_until_open();
    let entry = private_file::PrivateFileEntry::ensure(&target).unwrap();
    publish_absent(&entry, &target, TRUSTED_BYTES);
    waiter.release_and_join();
    assert_eq!(std::fs::read(target).unwrap(), TRUSTED_BYTES);
}

#[test]
fn recycled_slot_open_before_lock_never_truncates_the_published_target() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join(TARGET_NAME);
    let entry = private_file::PrivateFileEntry::ensure(&target).unwrap();
    let temporary = entry.temporary(TEMPORARY_PREFIX, TEMPORARY_SUFFIX).unwrap();
    entry.remove_temporary(&temporary).unwrap();
    drop(temporary);
    let waiter = spawn_stage_waiter(target.clone());
    waiter.wait_until_open();
    publish_absent(&entry, &target, TRUSTED_BYTES);
    waiter.release_and_join();
    assert_eq!(std::fs::read(target).unwrap(), TRUSTED_BYTES);
}

#[test]
fn target_open_before_lock_restarts_from_the_replacement_snapshot() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join(TARGET_NAME);
    atomic_write_json(&target, &json!({"state": ORIGINAL_STATE})).unwrap();
    let waiter = spawn_target_waiter(target.clone());
    waiter.wait_until_open();
    atomic_write_json(&target, &json!({"state": REPLACEMENT_STATE})).unwrap();
    waiter.release_and_join();
    let actual: serde_json::Value = read_json(&target).unwrap();
    assert_eq!(actual["state"], REPLACEMENT_STATE);
}

fn publish_absent(entry: &private_file::PrivateFileEntry, target: &std::path::Path, bytes: &[u8]) {
    let mut temporary = entry.temporary(TEMPORARY_PREFIX, TEMPORARY_SUFFIX).unwrap();
    temporary.file.write_all(bytes).unwrap();
    temporary.sync_and_inspect(target, u64::MAX).unwrap();
    entry.rename_temporary(&temporary, u64::MAX).unwrap();
}

struct PausedWaiter {
    opened: mpsc::Receiver<()>,
    release: mpsc::Sender<()>,
    thread: std::thread::JoinHandle<anyhow::Result<()>>,
}

impl PausedWaiter {
    fn wait_until_open(&self) {
        self.opened.recv_timeout(RACE_TIMEOUT).unwrap();
    }

    fn release_and_join(self) {
        self.release.send(()).unwrap();
        self.thread.join().unwrap().unwrap();
    }
}

fn spawn_stage_waiter(target: std::path::PathBuf) -> PausedWaiter {
    spawn_waiter(target, false)
}

fn spawn_target_waiter(target: std::path::PathBuf) -> PausedWaiter {
    spawn_waiter(target, true)
}

fn spawn_waiter(target: std::path::PathBuf, target_hook: bool) -> PausedWaiter {
    let (opened_sender, opened) = mpsc::channel();
    let (release, release_receiver) = mpsc::channel();
    let thread = std::thread::spawn(move || {
        let entry = private_file::PrivateFileEntry::ensure(&target)?;
        let mut opened_sender = Some(opened_sender);
        let mut hook = || {
            if let Some(sender) = opened_sender.take() {
                sender.send(())?;
                release_receiver.recv_timeout(RACE_TIMEOUT)?;
            }
            Ok(())
        };
        let temporary = if target_hook {
            entry.temporary_with_target_open_hook(TEMPORARY_PREFIX, TEMPORARY_SUFFIX, &mut hook)?
        } else {
            entry.temporary_with_open_hook(TEMPORARY_PREFIX, TEMPORARY_SUFFIX, &mut hook)?
        };
        entry.remove_temporary(&temporary)
    });
    PausedWaiter {
        opened,
        release,
        thread,
    }
}
