use super::*;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;

const PROBE_LATE_OUTPUT: &[u8] = b"probe-late-output";

#[test]
fn reader_probe_recovers_after_tentative_eof_identity_expires() {
    let (mut recovery, lifecycle, _slave_path) = reader_probe_fixture();
    assert!(lifecycle.kernel_eof_identity().is_none());

    let version = recovery_version(&mut recovery);
    let authority = recovery
        .acquire_kernel_eof_authority(&version)
        .unwrap()
        .unwrap();
    assert!(
        recovery
            .validates_kernel_eof_authority(&authority, &version)
            .unwrap()
    );
}

#[test]
fn late_output_revokes_a_reader_probe_before_commit() {
    let (mut recovery, _lifecycle, slave_path) = reader_probe_fixture();
    let version = recovery_version(&mut recovery);
    let authority = recovery
        .acquire_kernel_eof_authority(&version)
        .unwrap()
        .unwrap();
    let mut slave = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOCTTY)
        .open(slave_path)
        .unwrap();
    slave.write_all(PROBE_LATE_OUTPUT).unwrap();

    assert!(
        !recovery
            .validates_kernel_eof_authority(&authority, &version)
            .unwrap()
    );
}

#[test]
fn in_flight_reader_prevents_probe_acquisition() {
    let (mut recovery, lifecycle, _slave_path) = reader_probe_fixture();
    let read = lifecycle.begin_output_read().unwrap();
    let version = recovery_version(&mut recovery);
    assert!(
        recovery
            .acquire_kernel_eof_authority(&version)
            .unwrap()
            .is_none()
    );
    drop(read);
}

fn reader_probe_fixture() -> (PtyOutputRecovery, Arc<ReaderLifecycle>, PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let pair = open_test_pty();
    let slave_path = pair.master.tty_name().unwrap();
    let lifecycle = Arc::new(ReaderLifecycle::default());
    let mut exclusive =
        ExclusivePtyOutput::capture(pair.master.as_ref(), test_journal(directory.path())).unwrap();
    exclusive.bind_kernel_eof_for_test(pair.master.as_ref());
    let output = exclusive.complete_reader_activation(Arc::clone(&lifecycle));
    drop(pair.slave);
    (output.recovery(), lifecycle, slave_path)
}

fn recovery_version(recovery: &mut PtyOutputRecovery) -> ProcessClosureVersion {
    let binding = recovery.master_alias_authority.as_ref().unwrap();
    ProcessClosureVersion::for_pty_output_test(binding)
}
