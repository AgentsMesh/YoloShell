use super::empty_eof_fixture::EmptyEofFixture;
use super::{
    FinalizedProducersGoneReady, FreezePoll, FrozenProcessTree, FrozenProducersGoneCommit,
    FrozenProducersGoneFinalization, FrozenProducersGonePoll,
};
use crate::process::LiveProcessIdentity;
use crate::process::inventory_test_support::TestProcess;
use crate::process::kernel_eof_authority::{KernelPtyEofIdentity, bind_scripted_kernel_eof};
use std::sync::Arc;

const EXPECTED_PENDING_DETAIL: &str = "empty token fixture skipped confirmation";
const EXPECTED_FROZEN_DETAIL: &str = "empty token fixture stayed pending";
const EXPECTED_GONE_DETAIL: &str = "empty token fixture did not prepare gone";
const EXPECTED_REVALIDATION_DETAIL: &str = "empty token fixture did not require revalidation";
const INVALID_PROOF_ACCEPTED_DETAIL: &str = "invalid current PTY proof finalized empty gone";
const FOREIGN_FINALIZED_ACCEPTED_DETAIL: &str =
    "another process tree accepted a finalized producers-gone token";
const FINAL_LATE_SOURCE_ACCEPTED_DETAIL: &str =
    "late process source survived final output revalidation";
const FINALIZED_REQUIRED_DETAIL: &str = "empty producers-gone transaction did not finalize";

#[test]
fn stale_permit_cannot_prepare_empty_kernel_eof_gone() {
    let mut frozen = frozen_empty();
    let stale = crate::process::ProcessClosureVersion::default();
    let output = KernelPtyEofIdentity::from_kernel_read();
    let (permit, _) = bind_scripted_kernel_eof(&stale, &output);

    assert!(matches!(
        frozen
            .prepare_producers_gone_with_kernel_eof(permit)
            .unwrap(),
        FrozenProducersGonePoll::Refreeze(_)
    ));
}

#[test]
fn foreign_validated_eof_cannot_finalize_empty_gone() {
    let (mut frozen, ready, _validated, seal) = prepared_empty_commit();
    let version = frozen.closure_version();
    let foreign = KernelPtyEofIdentity::from_kernel_read();
    let (_, validation) = bind_scripted_kernel_eof(&version, &foreign);
    let foreign = validation.validate_scripted(&version, &foreign).unwrap();
    let foreign = foreign.bind_reopen_seal_for_test(&seal);

    assert!(
        frozen
            .finalize_producers_gone_commit(&ready, Some(foreign))
            .is_err()
    );
}

#[test]
fn empty_gone_cannot_skip_commit_preparation() {
    let mut frozen = frozen_empty();
    let version = frozen.closure_version();
    let output = KernelPtyEofIdentity::from_kernel_read();
    let (permit, _) = bind_scripted_kernel_eof(&version, &output);
    let ready = prepare_kernel_eof_gone(&mut frozen, permit);

    assert!(frozen.finalize_producers_gone_commit(&ready, None).is_err());
}

#[test]
fn foreign_committed_seal_cannot_finalize_empty_gone() {
    let (mut frozen, ready, validated, _local_seal) = prepared_empty_commit();
    let (foreign, _foreign_ready, _foreign_validated, foreign_seal) = prepared_empty_commit();
    drop(foreign);
    let validated = validated.bind_reopen_seal_for_test(&foreign_seal);

    assert!(
        frozen
            .finalize_producers_gone_commit(&ready, Some(validated))
            .is_err()
    );
}

#[test]
fn invalid_current_proof_cannot_finalize_empty_gone() {
    let (mut frozen, ready, validated, seal) = prepared_empty_commit();
    let validated = validated.bind_reopen_seal_for_test(&seal);
    crate::process::pty_session_proof::fail_next_reopen_seal_validation_for_test();

    let error = match frozen.finalize_producers_gone_commit(&ready, Some(validated)) {
        Ok(_) => panic!("{INVALID_PROOF_ACCEPTED_DETAIL}"),
        Err(error) => error,
    };
    assert!(FrozenProcessTree::kernel_eof_error_invalidates_output(
        &error
    ));
}

#[test]
fn late_source_after_output_revalidation_invalidates_kernel_eof() {
    let (mut frozen, ready, validated, seal) = prepared_empty_commit();
    let validated = validated.bind_reopen_seal_for_test(&seal);
    let source = TestProcess::spawn();
    let live = LiveProcessIdentity::capture(source.identity)
        .unwrap()
        .unwrap();
    let _ = frozen.tree.events.reconcile_inventory([live.clone()]);
    frozen.tree.events.queue_test_fork(live);

    assert!(
        matches!(
            frozen
                .finalize_producers_gone_commit(&ready, Some(validated))
                .unwrap(),
            FrozenProducersGoneFinalization::OutputInvalidated
        ),
        "{FINAL_LATE_SOURCE_ACCEPTED_DETAIL}"
    );
}

#[test]
fn finalized_empty_gone_token_cannot_commit_another_tree() {
    let (_foreign, token) = finalized_empty();
    let local = frozen_empty();
    let local_root = local.root();

    let failure = match local.commit_producers_gone(token) {
        Ok(_) => panic!("{FOREIGN_FINALIZED_ACCEPTED_DETAIL}"),
        Err(failure) => failure,
    };

    assert_eq!(failure.into_state().root(), local_root);
}

fn finalized_empty() -> (FrozenProcessTree, FinalizedProducersGoneReady) {
    let (mut frozen, ready, validated, seal) = prepared_empty_commit();
    let validated = validated.bind_reopen_seal_for_test(&seal);
    let FrozenProducersGoneFinalization::Finalized(finalized) = frozen
        .finalize_producers_gone_commit(&ready, Some(validated))
        .unwrap()
    else {
        panic!("{FINALIZED_REQUIRED_DETAIL}");
    };
    (frozen, finalized)
}

pub(super) fn prepared_empty_commit() -> (
    FrozenProcessTree,
    super::FrozenProducersGoneReady,
    crate::process::kernel_eof_authority::ValidatedKernelEof,
    crate::process::CommittedPtyReopenSealAuthority,
) {
    let mut frozen = frozen_empty();
    let version = frozen.closure_version();
    let stale = KernelPtyEofIdentity::from_kernel_read();
    let (permit, _) = bind_scripted_kernel_eof(&version, &stale);
    let ready = prepare_kernel_eof_gone(&mut frozen, permit);
    assert!(matches!(
        frozen.prepare_producers_gone_commit(&ready).unwrap(),
        FrozenProducersGoneCommit::FreshEofRequired
    ));
    let output = KernelPtyEofIdentity::from_kernel_read();
    assert!(!Arc::ptr_eq(&stale, &output));
    let (permit, validation) = bind_scripted_kernel_eof(&version, &output);
    let ready = prepare_kernel_eof_gone(&mut frozen, permit);
    let FrozenProducersGoneCommit::ReopenSealed(seal) =
        frozen.prepare_producers_gone_commit(&ready).unwrap()
    else {
        panic!("{EXPECTED_REVALIDATION_DETAIL}");
    };
    let validated = validation.validate_scripted(&version, &output).unwrap();
    (frozen, ready, validated, seal)
}

pub(super) fn prepare_kernel_eof_gone(
    frozen: &mut FrozenProcessTree,
    permit: crate::process::kernel_eof_authority::KernelEofPermit,
) -> super::FrozenProducersGoneReady {
    match frozen
        .prepare_producers_gone_with_kernel_eof(permit)
        .unwrap()
    {
        FrozenProducersGonePoll::Gone(ready) => ready,
        _ => panic!("{EXPECTED_GONE_DETAIL}"),
    }
}

pub(super) fn frozen_empty() -> FrozenProcessTree {
    let (freezing, _pty) = EmptyEofFixture::exact_gone_source().into_prepared_freezing_parts();
    let mut freezing = match freezing.poll().unwrap() {
        FreezePoll::Pending(freezing) => freezing,
        FreezePoll::Frozen(_) => panic!("{EXPECTED_PENDING_DETAIL}"),
    };
    let ready = freezing
        .prepare_poll()
        .unwrap()
        .authorize_kernel_eof_for_test();
    match freezing.commit_poll(ready) {
        FreezePoll::Frozen(frozen) => frozen,
        FreezePoll::Pending(_) => panic!("{EXPECTED_FROZEN_DETAIL}"),
    }
}
