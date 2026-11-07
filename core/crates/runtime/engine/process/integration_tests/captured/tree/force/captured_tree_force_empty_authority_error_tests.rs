use super::empty_token_tests::{frozen_empty, prepare_kernel_eof_gone, prepared_empty_commit};
use super::{FrozenProcessTree, FrozenProducersGoneFinalization, FrozenProducersGonePoll};
use crate::process::kernel_eof_authority::{KernelPtyEofIdentity, bind_scripted_kernel_eof};

const UNTYPED_AUTHORITY_ERROR_DETAIL: &str =
    "post-seal process authority error did not invalidate output";
const STALE_PROCESS_AUTHORITY_DETAIL: &str =
    "post-seal process authority error did not force a refreeze";
const PREPARE_ERROR_REQUIRED_DETAIL: &str =
    "post-seal process authority failure unexpectedly prepared an empty commit";
const FINALIZE_ERROR_REQUIRED_DETAIL: &str =
    "post-validation process authority failure unexpectedly finalized an empty commit";

#[test]
fn prepare_authority_error_after_physical_seal_requires_fresh_eof() {
    let mut frozen = frozen_empty();
    let version = frozen.closure_version();
    let output = KernelPtyEofIdentity::from_kernel_read();
    let (permit, _) = bind_scripted_kernel_eof(&version, &output);
    let ready = prepare_kernel_eof_gone(&mut frozen, permit);
    frozen.fail_next_gone_authority_check_for_test();

    let error = match frozen.prepare_producers_gone_commit(&ready) {
        Err(error) => error,
        Ok(_) => panic!("{PREPARE_ERROR_REQUIRED_DETAIL}"),
    };

    assert_output_invalidated(&error);
    assert_refreeze_required(&mut frozen);
}

#[test]
fn final_authority_error_after_output_revalidation_requires_fresh_eof() {
    let (mut frozen, ready, validated, seal) = prepared_empty_commit();
    let validated = validated.bind_reopen_seal_for_test(&seal);
    frozen.fail_next_gone_authority_check_for_test();

    let error = match frozen.finalize_producers_gone_commit(&ready, Some(validated)) {
        Err(error) => error,
        Ok(
            FrozenProducersGoneFinalization::Finalized(_)
            | FrozenProducersGoneFinalization::OutputInvalidated
            | FrozenProducersGoneFinalization::Refreeze,
        ) => {
            panic!("{FINALIZE_ERROR_REQUIRED_DETAIL}")
        }
    };

    assert_output_invalidated(&error);
    assert_refreeze_required(&mut frozen);
}

fn assert_output_invalidated(error: &anyhow::Error) {
    assert!(
        FrozenProcessTree::kernel_eof_error_invalidates_output(error),
        "{UNTYPED_AUTHORITY_ERROR_DETAIL}: {error:#}"
    );
}

fn assert_refreeze_required(frozen: &mut FrozenProcessTree) {
    let permit = frozen.kernel_eof_permit_for_test();
    assert!(
        matches!(
            frozen
                .prepare_producers_gone_with_kernel_eof(permit)
                .unwrap(),
            FrozenProducersGonePoll::Refreeze(_)
        ),
        "{STALE_PROCESS_AUTHORITY_DETAIL}"
    );
}
