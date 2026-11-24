use super::*;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;

const LATE_EXCLUSIVE_OUTPUT: &[u8] = b"late-exclusive-output";
const SLAVE_OPEN_FLAGS: libc::c_int = libc::O_NOCTTY;

#[test]
fn exclusive_native_eio_issues_valid_kernel_eof_authority() {
    let directory = tempfile::tempdir().unwrap();
    let pair = open_test_pty();
    let mut output =
        ExclusivePtyOutput::capture(pair.master.as_ref(), test_journal(directory.path())).unwrap();
    let version = output.bind_kernel_eof_for_test(pair.master.as_ref());
    drop(pair.slave);

    let authority = output
        .acquire_kernel_eof_authority(&version)
        .unwrap()
        .unwrap();

    output.record_kernel_eof_progress(OutputProgressState::Quiescent);
    assert!(
        output
            .validates_kernel_eof_authority(&authority, &version)
            .unwrap()
    );
}

#[test]
fn exclusive_authority_rotates_after_late_output_and_a_new_eof() {
    let directory = tempfile::tempdir().unwrap();
    let pair = open_test_pty();
    let slave_path = pair.master.tty_name().unwrap();
    let journal = test_journal(directory.path());
    let mut output =
        ExclusivePtyOutput::capture(pair.master.as_ref(), Arc::clone(&journal)).unwrap();
    let version = output.bind_kernel_eof_for_test(pair.master.as_ref());
    drop(pair.slave);
    let stale = output
        .acquire_kernel_eof_authority(&version)
        .unwrap()
        .unwrap();
    let mut slave = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(SLAVE_OPEN_FLAGS)
        .open(slave_path)
        .unwrap();
    slave.write_all(LATE_EXCLUSIVE_OUTPUT).unwrap();

    assert!(
        !output
            .validates_kernel_eof_authority(&stale, &version)
            .unwrap()
    );
    assert!(
        output
            .acquire_kernel_eof_authority(&version)
            .unwrap()
            .is_none()
    );
    let replayed: Vec<u8> = journal
        .lock()
        .unwrap()
        .replay_after(u64::default(), LATE_EXCLUSIVE_OUTPUT.len())
        .unwrap()
        .into_iter()
        .flat_map(|chunk| chunk.data)
        .collect();
    assert_eq!(replayed, LATE_EXCLUSIVE_OUTPUT);
    drop(slave);
    let current = output
        .acquire_kernel_eof_authority(&version)
        .unwrap()
        .unwrap();
    assert!(
        output
            .validates_kernel_eof_authority(&current, &version)
            .unwrap()
    );
}

#[test]
fn exclusive_authority_is_revoked_by_a_silent_slave_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let pair = open_test_pty();
    let slave_path = pair.master.tty_name().unwrap();
    let mut output =
        ExclusivePtyOutput::capture(pair.master.as_ref(), test_journal(directory.path())).unwrap();
    let version = output.bind_kernel_eof_for_test(pair.master.as_ref());
    drop(pair.slave);
    let stale = output
        .acquire_kernel_eof_authority(&version)
        .unwrap()
        .unwrap();
    let slave = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(SLAVE_OPEN_FLAGS)
        .open(slave_path)
        .unwrap();

    assert!(
        !output
            .validates_kernel_eof_authority(&stale, &version)
            .unwrap()
    );
    drop(slave);
    let current = output
        .acquire_kernel_eof_authority(&version)
        .unwrap()
        .unwrap();
    assert!(
        !output
            .validates_kernel_eof_authority(&stale, &version)
            .unwrap()
    );
    assert!(
        output
            .validates_kernel_eof_authority(&current, &version)
            .unwrap()
    );
}

#[test]
fn exclusive_eof_rejects_foreign_proof_for_the_same_pty_object() {
    let exact_directory = tempfile::tempdir().unwrap();
    let foreign_directory = tempfile::tempdir().unwrap();
    let pair = open_test_pty();
    let mut output =
        ExclusivePtyOutput::capture(pair.master.as_ref(), test_journal(exact_directory.path()))
            .unwrap();
    let exact = output.bind_kernel_eof_for_test(pair.master.as_ref());
    let mut foreign_output =
        ExclusivePtyOutput::capture(pair.master.as_ref(), test_journal(foreign_directory.path()))
            .unwrap();
    let foreign = foreign_output.bind_kernel_eof_for_test(pair.master.as_ref());
    drop(foreign_output);
    drop(pair.slave);

    assert!(
        output
            .acquire_kernel_eof_authority(&foreign)
            .unwrap()
            .is_none()
    );
    let authority = output
        .acquire_kernel_eof_authority(&exact)
        .unwrap()
        .unwrap();
    assert!(
        output
            .validates_kernel_eof_authority(&authority, &exact)
            .unwrap()
    );
}

#[test]
fn exclusive_output_rejects_a_foreign_pty_object_binding() {
    let directory = tempfile::tempdir().unwrap();
    let exact_pair = open_test_pty();
    let foreign_pair = open_test_pty();
    let mut output =
        ExclusivePtyOutput::capture(exact_pair.master.as_ref(), test_journal(directory.path()))
            .unwrap();
    let (foreign, _foreign_version) =
        ExclusivePtyOutput::kernel_eof_binding_for_test(foreign_pair.master.as_ref());

    assert!(output.bind_master_alias_authority(foreign).is_err());
    let exact = output.bind_kernel_eof_for_test(exact_pair.master.as_ref());
    drop(exact_pair.slave);
    let authority = output
        .acquire_kernel_eof_authority(&exact)
        .unwrap()
        .unwrap();
    assert!(
        output
            .validates_kernel_eof_authority(&authority, &exact)
            .unwrap()
    );
}
