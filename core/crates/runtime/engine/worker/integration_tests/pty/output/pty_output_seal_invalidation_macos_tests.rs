use super::*;
use crate::process::{CommittedPtyReopenSealAuthority, FrozenProcessTree};
use crate::worker::pty_output_progress::{
    KernelEofCommitOutcome, KernelEofValidation, KernelEofValidationRequest,
};
use std::panic::{AssertUnwindSafe, catch_unwind};

const GENERIC_COMMIT_FAILURE_DETAIL: &str = "injected sealed EOF commit failure";
const PANICKED_COMMIT_DETAIL: &str = "injected sealed EOF commit panic";

#[test]
fn typed_seal_failure_revokes_reader_epoch_and_requires_a_fresh_eof() {
    let (_directory, mut recovery, lifecycle, version) = reader_fixture();
    let stale = retain_eof(&lifecycle);
    let mut authority = recovery
        .acquire_kernel_eof_authority(&version)
        .unwrap()
        .unwrap();

    let error = recovery
        .commit_if_kernel_eof_current(&mut authority, &version, |validate| {
            fail_after_seal(validate, Failure::Typed)
        })
        .unwrap_err();

    assert!(FrozenProcessTree::kernel_eof_error_invalidates_output(
        &error
    ));
    assert!(lifecycle.kernel_eof_observation().is_none());
    assert!(lifecycle.output_reader_wakeup.load(Ordering::Acquire));
    assert!(
        !recovery
            .validates_kernel_eof_authority(&authority, &version)
            .unwrap()
    );
    let fresh = retain_eof(&lifecycle);
    assert!(!Arc::ptr_eq(&stale, &fresh));
}

#[test]
fn sealed_probe_error_revokes_probe_and_requires_a_fresh_identity() {
    let (_directory, mut recovery, lifecycle, version) = reader_fixture();
    let mut authority = recovery
        .acquire_kernel_eof_authority(&version)
        .unwrap()
        .unwrap();
    let stale = authority.identity_for_test();

    assert!(
        recovery
            .commit_if_kernel_eof_current(&mut authority, &version, |validate| {
                fail_after_seal(validate, Failure::Generic)
            })
            .is_err()
    );

    assert!(lifecycle.kernel_eof_observation().is_none());
    let fresh = recovery
        .acquire_kernel_eof_authority(&version)
        .unwrap()
        .unwrap();
    assert!(!Arc::ptr_eq(&stale, &fresh.identity_for_test()));
}

#[test]
fn typed_seal_failure_revokes_exclusive_epoch_and_requires_a_fresh_eof() {
    let (_directory, mut output, version) = exclusive_fixture();
    let mut authority = output
        .acquire_kernel_eof_authority(&version)
        .unwrap()
        .unwrap();
    let stale = authority.identity_for_test();

    let error = output
        .commit_if_kernel_eof_current(&mut authority, &version, |validate| {
            fail_after_seal(validate, Failure::Typed)
        })
        .unwrap_err();

    assert!(FrozenProcessTree::kernel_eof_error_invalidates_output(
        &error
    ));
    assert!(output.kernel_eof_identity_for_test().is_none());
    assert!(
        !output
            .validates_kernel_eof_authority(&authority, &version)
            .unwrap()
    );
    let fresh = output
        .acquire_kernel_eof_authority(&version)
        .unwrap()
        .unwrap();
    assert!(!Arc::ptr_eq(&stale, &fresh.identity_for_test()));
}

#[test]
fn sealed_exclusive_panic_revokes_epoch_before_unwinding() {
    let (_directory, mut output, version) = exclusive_fixture();
    let mut authority = output
        .acquire_kernel_eof_authority(&version)
        .unwrap()
        .unwrap();

    let panic = catch_unwind(AssertUnwindSafe(|| {
        output.commit_if_kernel_eof_current(&mut authority, &version, |validate| {
            fail_after_seal(validate, Failure::Panic)
        })
    }));

    assert!(panic.is_err());
    assert!(output.kernel_eof_identity_for_test().is_none());
    assert!(
        output
            .acquire_kernel_eof_authority(&version)
            .unwrap()
            .is_some()
    );
}

#[derive(Clone, Copy)]
enum Failure {
    Typed,
    Generic,
    Panic,
}

fn fail_after_seal(
    validate: &mut dyn FnMut(
        KernelEofValidationRequest,
    ) -> anyhow::Result<Option<KernelEofValidation>>,
    failure: Failure,
) -> anyhow::Result<KernelEofCommitOutcome> {
    let Some(KernelEofValidation::Current(validated)) =
        validate(KernelEofValidationRequest::Current)?
    else {
        return Ok(KernelEofCommitOutcome::Rejected);
    };
    let seal = CommittedPtyReopenSealAuthority::missing_proof_for_test();
    let Some(KernelEofValidation::ReopenSealed(_)) =
        validate(KernelEofValidationRequest::ReopenSealed { validated, seal })?
    else {
        return Ok(KernelEofCommitOutcome::Rejected);
    };
    match failure {
        Failure::Typed => Err(FrozenProcessTree::kernel_eof_invalidating_error_for_test()),
        Failure::Generic => anyhow::bail!(GENERIC_COMMIT_FAILURE_DETAIL),
        Failure::Panic => panic!("{PANICKED_COMMIT_DETAIL}"),
    }
}

fn reader_fixture() -> (
    tempfile::TempDir,
    PtyOutputRecovery,
    Arc<ReaderLifecycle>,
    ProcessClosureVersion,
) {
    let directory = tempfile::tempdir().unwrap();
    let pair = open_test_pty();
    let lifecycle = Arc::new(ReaderLifecycle::default());
    let mut exclusive =
        ExclusivePtyOutput::capture(pair.master.as_ref(), test_journal(directory.path())).unwrap();
    let version = exclusive.bind_kernel_eof_for_test(pair.master.as_ref());
    let output = exclusive.complete_reader_activation(Arc::clone(&lifecycle));
    drop(pair.slave);
    (directory, output.recovery(), lifecycle, version)
}

fn exclusive_fixture() -> (tempfile::TempDir, ExclusivePtyOutput, ProcessClosureVersion) {
    let directory = tempfile::tempdir().unwrap();
    let pair = open_test_pty();
    let mut output =
        ExclusivePtyOutput::capture(pair.master.as_ref(), test_journal(directory.path())).unwrap();
    let version = output.bind_kernel_eof_for_test(pair.master.as_ref());
    drop(pair.slave);
    (directory, output, version)
}

fn retain_eof(lifecycle: &ReaderLifecycle) -> Arc<KernelPtyEofIdentity> {
    let publication = lifecycle
        .begin_output_read()
        .unwrap()
        .kernel_eof_for_test()
        .await_eof_publication();
    drop(publication);
    lifecycle.kernel_eof_identity().unwrap()
}
