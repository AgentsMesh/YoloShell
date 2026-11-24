use super::*;
use crate::worker::PtyReadOutcome;
use std::os::unix::fs::OpenOptionsExt;

const SILENT_REOPEN_EOF_DETAIL: &str = "silent slave reopen did not interrupt kernel EOF";

#[test]
fn silent_slave_reopen_requires_a_fresh_reader_eof_identity() {
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
    let stale_publication = lifecycle
        .begin_output_read()
        .unwrap()
        .read_from(reader.as_mut(), &mut byte)
        .expect_kernel_eof()
        .await_eof_publication();
    drop(stale_publication);
    let stale = recovery
        .acquire_kernel_eof_authority(&version)
        .unwrap()
        .unwrap();
    let stale_identity = stale.identity_for_test();
    let reopened = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOCTTY)
        .open(slave_path)
        .unwrap();

    let outcome = lifecycle
        .begin_output_read()
        .unwrap()
        .read_from(reader.as_mut(), &mut byte);
    let PtyReadOutcome::Failed { attempt, error } = outcome else {
        panic!("{SILENT_REOPEN_EOF_DETAIL}");
    };
    assert_eq!(error.kind(), std::io::ErrorKind::WouldBlock);
    attempt.mark_quiescent().unwrap();
    assert!(
        !recovery
            .validates_kernel_eof_authority(&stale, &version)
            .unwrap()
    );
    drop(reopened);

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
    assert!(!Arc::ptr_eq(&stale_identity, &current.identity_for_test()));
    current_publication.publish();
}
