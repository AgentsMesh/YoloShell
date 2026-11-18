use super::*;
use crate::process::CommittedPtyReopenSealAuthority;
use crate::worker::pty_output_progress::{
    KernelEofCommitOutcome, KernelEofValidation, KernelEofValidationRequest,
};
use std::fs::File;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

const SLAVE_OPEN_FLAGS: libc::c_int = libc::O_NOCTTY;

pub(super) fn reject_after_reopen(
    validate: &mut dyn FnMut(
        KernelEofValidationRequest,
    ) -> anyhow::Result<Option<KernelEofValidation>>,
    path: &Path,
    reopened: &mut Option<File>,
) -> anyhow::Result<KernelEofCommitOutcome> {
    let Some(KernelEofValidation::Current(validated)) =
        validate(KernelEofValidationRequest::Current)?
    else {
        return Ok(KernelEofCommitOutcome::Rejected);
    };
    *reopened = Some(open_slave(path));
    let seal = CommittedPtyReopenSealAuthority::missing_proof_for_test();
    assert!(validate(KernelEofValidationRequest::ReopenSealed { validated, seal })?.is_none());
    Ok(KernelEofCommitOutcome::Rejected)
}

pub(super) fn install_reader_seal(
    output: &mut PtyOutputRecovery,
    version: &ProcessClosureVersion,
) -> Arc<KernelPtyEofIdentity> {
    let mut authority = output
        .acquire_kernel_eof_authority(version)
        .unwrap()
        .unwrap();
    let stale = authority.identity_for_test();
    assert!(
        !output
            .commit_if_kernel_eof_current(&mut authority, version, |validate| {
                seal_and_reject(validate)
            })
            .unwrap()
    );
    stale
}

pub(super) fn install_exclusive_seal(
    output: &mut ExclusivePtyOutput,
    version: &ProcessClosureVersion,
) -> Arc<KernelPtyEofIdentity> {
    let mut authority = output
        .acquire_kernel_eof_authority(version)
        .unwrap()
        .unwrap();
    let stale = authority.identity_for_test();
    assert!(
        !output
            .commit_if_kernel_eof_current(&mut authority, version, |validate| {
                seal_and_reject(validate)
            })
            .unwrap()
    );
    stale
}

fn seal_and_reject(
    validate: &mut dyn FnMut(
        KernelEofValidationRequest,
    ) -> anyhow::Result<Option<KernelEofValidation>>,
) -> anyhow::Result<KernelEofCommitOutcome> {
    let Some(KernelEofValidation::Current(validated)) =
        validate(KernelEofValidationRequest::Current)?
    else {
        return Ok(KernelEofCommitOutcome::Rejected);
    };
    let seal = CommittedPtyReopenSealAuthority::missing_proof_for_test();
    assert!(matches!(
        validate(KernelEofValidationRequest::ReopenSealed { validated, seal })?,
        Some(KernelEofValidation::ReopenSealed(_))
    ));
    Ok(KernelEofCommitOutcome::Rejected)
}

pub(super) fn assert_fresh_reader_identity(
    output: &mut PtyOutputRecovery,
    version: &ProcessClosureVersion,
    stale: &Arc<KernelPtyEofIdentity>,
) {
    let fresh = output
        .acquire_kernel_eof_authority(version)
        .unwrap()
        .unwrap();
    assert!(!Arc::ptr_eq(stale, &fresh.identity_for_test()));
}

pub(super) fn reader_fixture(
    retain_eof: bool,
) -> (
    tempfile::TempDir,
    PtyOutputRecovery,
    Arc<ReaderLifecycle>,
    ProcessClosureVersion,
    PathBuf,
) {
    let directory = tempfile::tempdir().unwrap();
    let pair = open_test_pty();
    let path = pair.master.tty_name().unwrap();
    let lifecycle = Arc::new(ReaderLifecycle::default());
    let mut exclusive =
        ExclusivePtyOutput::capture(pair.master.as_ref(), test_journal(directory.path())).unwrap();
    let version = exclusive.bind_kernel_eof_for_test(pair.master.as_ref());
    let output = exclusive.complete_reader_activation(Arc::clone(&lifecycle));
    drop(pair.slave);
    if retain_eof {
        let publication = lifecycle
            .begin_output_read()
            .unwrap()
            .kernel_eof_for_test()
            .await_eof_publication();
        drop(publication);
    }
    (directory, output.recovery(), lifecycle, version, path)
}

pub(super) fn exclusive_fixture() -> (
    tempfile::TempDir,
    ExclusivePtyOutput,
    ProcessClosureVersion,
    PathBuf,
) {
    let directory = tempfile::tempdir().unwrap();
    let pair = open_test_pty();
    let path = pair.master.tty_name().unwrap();
    let mut output =
        ExclusivePtyOutput::capture(pair.master.as_ref(), test_journal(directory.path())).unwrap();
    let version = output.bind_kernel_eof_for_test(pair.master.as_ref());
    drop(pair.slave);
    (directory, output, version, path)
}

pub(super) fn open_slave(path: &Path) -> File {
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(SLAVE_OPEN_FLAGS)
        .open(path)
        .unwrap()
}
