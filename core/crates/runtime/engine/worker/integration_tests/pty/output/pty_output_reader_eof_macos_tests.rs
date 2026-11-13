use super::*;
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;

const LATE_OUTPUT: &[u8] = b"late-reader-output";

#[test]
fn native_reader_without_runtime_custody_defers_tracked_settlement() {
    let directory = tempfile::tempdir().unwrap();
    let pair = open_test_pty();
    let lifecycle = Arc::new(ReaderLifecycle::default());
    let mut exclusive =
        ExclusivePtyOutput::capture(pair.master.as_ref(), test_journal(directory.path())).unwrap();
    exclusive.bind_kernel_eof_for_test(pair.master.as_ref());
    let output = exclusive.complete_reader_activation(lifecycle);

    assert!(!output.supports_tracked_settlement());
}

#[test]
fn revoked_reader_eof_authority_cannot_validate_against_a_new_eof() {
    let directory = tempfile::tempdir().unwrap();
    let pair = open_test_pty();
    let mut reader = pair.master.try_clone_reader().unwrap();
    let lifecycle = Arc::new(ReaderLifecycle::default());
    let mut exclusive =
        ExclusivePtyOutput::capture(pair.master.as_ref(), test_journal(directory.path())).unwrap();
    let version = exclusive.bind_kernel_eof_for_test(pair.master.as_ref());
    let output = exclusive.complete_reader_activation(Arc::clone(&lifecycle));
    let mut recovery = output.recovery();
    drop(pair.slave);
    let mut byte = [u8::default(); 1];
    let publication = lifecycle
        .begin_output_read()
        .unwrap()
        .read_from(reader.as_mut(), &mut byte)
        .expect_kernel_eof()
        .await_eof_publication();
    let stale = recovery
        .acquire_kernel_eof_authority(&version)
        .unwrap()
        .unwrap();
    drop(publication);
    // Repeated nonblocking reads after EOF are kernel-timing dependent. This
    // assertion needs a distinct sealed EOF identity, not a second OS read.
    let current_publication = lifecycle
        .begin_output_read()
        .unwrap()
        .kernel_eof_for_test()
        .await_eof_publication();

    assert!(
        !recovery
            .validates_kernel_eof_authority(&stale, &version)
            .unwrap()
    );
    let current = recovery
        .acquire_kernel_eof_authority(&version)
        .unwrap()
        .unwrap();
    current_publication.publish();
    assert!(
        recovery
            .validates_kernel_eof_authority(&current, &version)
            .unwrap()
    );
}

#[test]
fn reader_authority_is_revoked_before_late_output_is_staged() {
    let directory = tempfile::tempdir().unwrap();
    let pair = open_test_pty();
    let slave_path = pair.master.tty_name().unwrap();
    let mut reader = pair.master.try_clone_reader().unwrap();
    let lifecycle = Arc::new(ReaderLifecycle::default());
    let mut exclusive =
        ExclusivePtyOutput::capture(pair.master.as_ref(), test_journal(directory.path())).unwrap();
    let version = exclusive.bind_kernel_eof_for_test(pair.master.as_ref());
    let output = exclusive.complete_reader_activation(Arc::clone(&lifecycle));
    let mut recovery = output.recovery();
    drop(pair.slave);
    let mut byte = [u8::default(); 1];
    let publication = lifecycle
        .begin_output_read()
        .unwrap()
        .read_from(reader.as_mut(), &mut byte)
        .expect_kernel_eof()
        .await_eof_publication();
    let stale = recovery
        .acquire_kernel_eof_authority(&version)
        .unwrap()
        .unwrap();

    let mut slave = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOCTTY)
        .open(slave_path)
        .unwrap();
    slave.write_all(LATE_OUTPUT).unwrap();

    assert!(
        !recovery
            .validates_kernel_eof_authority(&stale, &version)
            .unwrap()
    );
    assert!(lifecycle.kernel_eof_identity().is_none());
    drop(publication);
    let mut received = [u8::default(); LATE_OUTPUT.len()];
    reader.read_exact(&mut received).unwrap();
    lifecycle
        .begin_output_read()
        .unwrap()
        .stage_for_test(&received)
        .unwrap();
    assert_eq!(
        lifecycle.lock_progress().pending.as_ref().unwrap().data,
        LATE_OUTPUT
    );
    lifecycle.clear_pending_output_for_test();
    drop(slave);
    let mut byte = [u8::default(); 1];
    let current_publication = lifecycle
        .begin_output_read()
        .unwrap()
        .read_from(reader.as_mut(), &mut byte)
        .expect_kernel_eof()
        .await_eof_publication();
    let current = recovery
        .acquire_kernel_eof_authority(&version)
        .unwrap()
        .unwrap();
    assert!(
        recovery
            .validates_kernel_eof_authority(&current, &version)
            .unwrap()
    );
    current_publication.publish();
}
