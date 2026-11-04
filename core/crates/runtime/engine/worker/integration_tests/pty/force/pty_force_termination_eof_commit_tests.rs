const EOF_VALIDATION_FAILURE_DETAIL: &str = "injected commit-time EOF validation failure";
const EOF_VALIDATION_PANIC_DETAIL: &str = "injected commit-time EOF validation panic";
const EXPECTED_EOF_VALIDATION_PANIC_DETAIL: &str = "commit-time EOF validation did not panic";
const RETAINED_EOF_TRANSACTION_DETAIL: &str =
    "commit failure did not retain the prepared kernel EOF transaction";
const RETIRED_EOF_TRANSACTION_DETAIL: &str =
    "successful retry did not retire the prepared kernel EOF transaction";
const INITIAL_ATTEMPT_AND_RETRY_COUNT: usize = OPERATION_CALL_INCREMENT + OPERATION_CALL_INCREMENT;

#[test]
fn kernel_eof_commit_revocation_preserves_frozen_custody() {
    let (_child, tree) = exited_empty_closure_fixture();
    let root = tree.root();
    let mut custody = Some(PtyProcessCustody::tracking(tree));
    let mut force = PtyForceTermination::requiring_empty_closure_output_for_test(
        EmptyClosureOutputRequirement::KernelEof,
    );
    let mut output = ScriptedForceOutput::new(FenceBehavior::Pending);
    drive_to_frozen(&mut force, &mut custody, &mut output);
    output.observe_kernel_eof();
    let ready = prepare_kernel_eof_gone_ready(&mut force, &mut custody, &mut output);
    output.revoke_eof_during_final_revalidation();
    assert_pending(commit_prepared_force(
        &mut force,
        &mut custody,
        &mut output,
        ready,
    ));
    output.assert_final_eof_revalidation_reached();
    output.assert_final_eof_token_not_minted();
    assert!(force.frozen_gone.is_none());

    assert_variant_root(&custody, CustodyKind::Frozen, root);
    deny_frozen_destructive_authority(&mut custody);
    output.observe_kernel_eof();
    drive_until(
        &mut force,
        &mut custody,
        &mut output,
        CustodyKind::GonePending,
    );
}

#[test]
fn kernel_eof_commit_validation_error_preserves_frozen_custody() {
    let (_child, tree) = exited_empty_closure_fixture();
    let root = tree.root();
    let mut custody = Some(PtyProcessCustody::tracking(tree));
    let mut force = PtyForceTermination::requiring_empty_closure_output_for_test(
        EmptyClosureOutputRequirement::KernelEof,
    );
    let mut output = ScriptedForceOutput::new(FenceBehavior::Pending);
    drive_to_frozen(&mut force, &mut custody, &mut output);
    output.observe_kernel_eof();
    let ready = prepare_kernel_eof_gone_ready(&mut force, &mut custody, &mut output);
    output.fail_after_final_eof_token_mint(EOF_VALIDATION_FAILURE_DETAIL);

    let error = expect_error(commit_prepared_force(
        &mut force,
        &mut custody,
        &mut output,
        ready,
    ));

    assert_error_contains(&error, EOF_VALIDATION_FAILURE_DETAIL);
    output.assert_final_eof_revalidation_reached();
    output.assert_final_eof_token_minted();
    assert_variant_root(&custody, CustodyKind::Frozen, root);
    assert!(
        force.frozen_gone.is_some(),
        "{RETAINED_EOF_TRANSACTION_DETAIL}"
    );
    let acquire_calls = output.eof_acquire_calls;

    assert_pending(poll_force(&mut force, &mut custody, &mut output));

    assert_variant_root(&custody, CustodyKind::GonePending, root);
    assert!(
        force.frozen_gone.is_none(),
        "{RETIRED_EOF_TRANSACTION_DETAIL}"
    );
    assert_eq!(output.eof_acquire_calls, acquire_calls);
    assert_eq!(
        output.final_eof_revalidation_calls,
        INITIAL_ATTEMPT_AND_RETRY_COUNT
    );
    assert_eq!(
        output.final_eof_token_mints,
        INITIAL_ATTEMPT_AND_RETRY_COUNT
    );
}

#[test]
fn kernel_eof_commit_validation_panic_preserves_frozen_custody() {
    let (_child, tree) = exited_empty_closure_fixture();
    let root = tree.root();
    let mut custody = Some(PtyProcessCustody::tracking(tree));
    let mut force = PtyForceTermination::requiring_empty_closure_output_for_test(
        EmptyClosureOutputRequirement::KernelEof,
    );
    let mut output = ScriptedForceOutput::new(FenceBehavior::Pending);
    drive_to_frozen(&mut force, &mut custody, &mut output);
    output.observe_kernel_eof();
    let ready = prepare_kernel_eof_gone_ready(&mut force, &mut custody, &mut output);
    output.panic_after_final_eof_token_mint(EOF_VALIDATION_PANIC_DETAIL);

    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = commit_prepared_force(&mut force, &mut custody, &mut output, ready);
    }));

    assert!(panic.is_err(), "{EXPECTED_EOF_VALIDATION_PANIC_DETAIL}");
    output.assert_final_eof_revalidation_reached();
    output.assert_final_eof_token_minted();
    assert_variant_root(&custody, CustodyKind::Frozen, root);
    assert!(
        force.frozen_gone.is_some(),
        "{RETAINED_EOF_TRANSACTION_DETAIL}"
    );
    let acquire_calls = output.eof_acquire_calls;

    assert_pending(poll_force(&mut force, &mut custody, &mut output));

    assert_variant_root(&custody, CustodyKind::GonePending, root);
    assert!(
        force.frozen_gone.is_none(),
        "{RETIRED_EOF_TRANSACTION_DETAIL}"
    );
    assert_eq!(output.eof_acquire_calls, acquire_calls);
    assert_eq!(
        output.final_eof_revalidation_calls,
        INITIAL_ATTEMPT_AND_RETRY_COUNT
    );
    assert_eq!(
        output.final_eof_token_mints,
        INITIAL_ATTEMPT_AND_RETRY_COUNT
    );
}
