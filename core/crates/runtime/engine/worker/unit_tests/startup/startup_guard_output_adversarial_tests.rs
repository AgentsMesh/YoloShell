use super::super::process_assertions::assert_process_tree_exits;
use super::super::tests::{
    TEST_MARKER_FILE, spawn_test_guard, wait_for_descendant, wait_for_tracked_identity,
};
use super::*;
use crate::worker::pty_output_progress::ReaderBackedPtyOutput;
use crate::worker::pty_reader_activation::PendingPtyReader;
use crate::worker::termination_timing::CapabilitySettlementDeferred;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

const OBSERVER_PROGRESS_POLL: Duration = Duration::from_millis(1);
const OBSERVER_PROGRESS_TIMEOUT: Duration = Duration::from_secs(2);
const REQUIRED_PROGRESS_CALLS: usize = 1;
const TEST_EXIT_CODE: u32 = 0;

#[derive(Debug)]
struct SettledChild;

impl portable_pty::ChildKiller for SettledChild {
    fn kill(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    fn clone_killer(&self) -> Box<dyn portable_pty::ChildKiller + Send + Sync> {
        Box::new(Self)
    }
}

impl portable_pty::Child for SettledChild {
    fn try_wait(&mut self) -> std::io::Result<Option<portable_pty::ExitStatus>> {
        Ok(Some(portable_pty::ExitStatus::with_exit_code(
            TEST_EXIT_CODE,
        )))
    }

    fn wait(&mut self) -> std::io::Result<portable_pty::ExitStatus> {
        Ok(portable_pty::ExitStatus::with_exit_code(TEST_EXIT_CODE))
    }

    fn process_id(&self) -> Option<u32> {
        None
    }
}

#[test]
fn prepared_reader_activation_panics_if_output_authority_is_displaced() {
    let observed = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut guard = StartupChildGuard::with_output(
        Box::new(SettledChild),
        ExclusivePtyOutput::observed(observed),
    );
    let (reader, receiver) = PendingPtyReader::for_test();
    let prepared = guard.prepare_reader_activation(reader).unwrap();
    guard.output_progress = None;

    let panic = catch_unwind(AssertUnwindSafe(|| {
        guard.activate_prepared_reader(prepared)
    }));

    assert!(panic.is_err());
    drop(receiver);
    guard.clear();
}

#[test]
fn untracked_reader_backed_cleanup_reaches_the_reader_recovery_path() {
    let mut guard = StartupChildGuard::with_reader_backed_output(
        Box::new(SettledChild),
        ReaderBackedPtyOutput::for_test(),
    );

    let _ = guard.terminate_with_owned_output();

    assert!(!guard.requires_cleanup());
}

#[test]
fn exclusive_cleanup_retries_while_the_tree_has_an_observer() {
    assert_observer_retry(false);
}

#[test]
fn reader_backed_cleanup_retries_while_the_tree_has_an_observer() {
    assert_observer_retry(true);
}

fn assert_observer_retry(reader_backed: bool) {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join(TEST_MARKER_FILE);
    let (mut guard, _master, root) = spawn_test_guard(&marker);
    let descendant = wait_for_descendant(&marker);
    wait_for_tracked_identity(&guard, descendant);
    let progress = std::sync::Arc::new(AtomicUsize::new(0));
    let custody = guard.fault_reaper.as_ref().unwrap().state_observer();
    guard.output_progress = Some(StartupOutputProgress::Exclusive(
        ExclusivePtyOutput::observed(std::sync::Arc::clone(&progress)),
    ));
    if reader_backed {
        guard.install_reader_backed_output_for_test();
    }
    let observer = std::sync::Arc::clone(guard.tree.as_ref().unwrap());
    let release_progress = std::sync::Arc::clone(&progress);
    let release = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + OBSERVER_PROGRESS_TIMEOUT;
        while release_progress.load(Ordering::Acquire) < REQUIRED_PROGRESS_CALLS {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(OBSERVER_PROGRESS_POLL);
        }
        drop(observer);
    });

    let result = guard.terminate_with_owned_output();

    release.join().unwrap();
    let error = result.unwrap_err();
    assert!(error.is::<CapabilitySettlementDeferred>());
    assert!(!guard.requires_cleanup());
    let status = custody.status().unwrap();
    assert!(status.custody_retained);
    assert!(!status.observer_only);
    assert!(progress.load(Ordering::Acquire) >= REQUIRED_PROGRESS_CALLS);
    assert_process_tree_exits(root, descendant);
}
