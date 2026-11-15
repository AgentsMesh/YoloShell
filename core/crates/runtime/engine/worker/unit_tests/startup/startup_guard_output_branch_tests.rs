use super::super::process_assertions::assert_process_tree_exits;
use super::super::tests::{
    TEST_MARKER_FILE, spawn_test_guard, wait_for_descendant, wait_for_tracked_identity,
};
use super::*;
use crate::worker::pty_reader_activation::PendingPtyReader;
use std::io;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

const INJECTED_PROGRESS_FAILURE: &str = "injected startup output progress failure";
const TEST_EXIT_CODE: u32 = 0;

#[derive(Debug)]
struct SettledChild;

impl portable_pty::ChildKiller for SettledChild {
    fn kill(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn clone_killer(&self) -> Box<dyn portable_pty::ChildKiller + Send + Sync> {
        Box::new(Self)
    }
}

impl portable_pty::Child for SettledChild {
    fn try_wait(&mut self) -> io::Result<Option<portable_pty::ExitStatus>> {
        Ok(Some(portable_pty::ExitStatus::with_exit_code(
            TEST_EXIT_CODE,
        )))
    }

    fn wait(&mut self) -> io::Result<portable_pty::ExitStatus> {
        Ok(portable_pty::ExitStatus::with_exit_code(TEST_EXIT_CODE))
    }

    fn process_id(&self) -> Option<u32> {
        None
    }
}

#[test]
fn reader_installation_covers_every_starting_authority() {
    let observed = Arc::new(AtomicUsize::new(usize::default()));
    let mut guard = StartupChildGuard::with_output(
        Box::new(SettledChild),
        ExclusivePtyOutput::observed(Arc::clone(&observed)),
    );

    guard.install_reader_backed_output_for_test();
    guard.install_reader_backed_output_for_test();
    guard.output_progress = None;
    guard.install_reader_backed_output_for_test();

    assert!(matches!(
        guard.output_progress,
        Some(StartupOutputProgress::ReaderBacked(_))
    ));
    guard.clear();
}

#[test]
fn activation_errors_preserve_or_reject_output_authority() {
    let observed = Arc::new(AtomicUsize::new(usize::default()));
    let mut guard = StartupChildGuard::with_output(
        Box::new(SettledChild),
        ExclusivePtyOutput::observed(Arc::clone(&observed)),
    );

    assert!(
        guard
            .activate_reader(PendingPtyReader::disconnected_for_test())
            .is_err()
    );
    assert!(matches!(
        guard.output_progress,
        Some(StartupOutputProgress::Exclusive(_))
    ));

    guard.install_reader_backed_output_for_test();
    let (reader, receiver) = PendingPtyReader::for_test();
    assert!(guard.activate_reader(reader).is_err());
    drop(receiver);

    guard.output_progress = None;
    let (reader, receiver) = PendingPtyReader::for_test();
    assert!(guard.activate_reader(reader).is_err());
    drop(receiver);
    guard.clear();
}

#[test]
fn preparation_rejects_missing_or_reader_backed_authority() {
    let mut guard = StartupChildGuard::with_reader_backed_output(
        Box::new(SettledChild),
        ReaderBackedPtyOutput::for_test(),
    );
    let (reader, receiver) = PendingPtyReader::for_test();
    assert!(guard.prepare_reader_activation(reader).is_err());
    drop(receiver);

    guard.output_progress = None;
    let (reader, receiver) = PendingPtyReader::for_test();
    assert!(guard.prepare_reader_activation(reader).is_err());
    drop(receiver);
    guard.clear();
}

#[test]
fn prepared_activation_releases_reader_only_after_output_commit() {
    let observed = Arc::new(AtomicUsize::new(usize::default()));
    let mut guard = StartupChildGuard::with_output(
        Box::new(SettledChild),
        ExclusivePtyOutput::observed(Arc::clone(&observed)),
    );
    let (reader, receiver) = PendingPtyReader::for_test();
    let prepared = guard.prepare_reader_activation(reader).unwrap();

    guard.activate_prepared_reader(prepared);

    receiver.recv().unwrap();
    assert!(matches!(
        guard.output_progress,
        Some(StartupOutputProgress::ReaderBacked(_))
    ));
    guard.clear();
}

#[test]
fn tracked_reader_backed_cleanup_reaps_the_complete_tree() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join(TEST_MARKER_FILE);
    let (mut guard, _master, root) = spawn_test_guard(&marker);
    let descendant = wait_for_descendant(&marker);
    wait_for_tracked_identity(&guard, descendant);
    guard.install_reader_backed_output_for_test();

    drop(guard);

    assert_process_tree_exits(root, descendant);
}

#[test]
fn retry_progress_retains_the_first_failure() {
    let mut guard = StartupChildGuard::new(Box::new(SettledChild));
    let mut failure = None;

    guard.progress_before_retry(&mut failure, || {
        Err(anyhow::anyhow!(INJECTED_PROGRESS_FAILURE))
    });
    guard.progress_before_retry(&mut failure, || Ok(OutputProgressState::Complete));

    assert!(
        failure
            .unwrap()
            .to_string()
            .contains(INJECTED_PROGRESS_FAILURE)
    );
    guard.clear();
}
