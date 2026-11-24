const NO_OUTPUT_FENCE_CALLS: usize = 0;
const FIRST_OUTPUT_FENCE_CALLS: usize = 1;
const SECOND_OUTPUT_FENCE_CALLS: usize = 2;
const EXPECTED_FROZEN_CUSTODY_DETAIL: &str = "force custody did not contain a frozen tree";

#[test]
fn empty_frozen_closure_reports_gone_without_waiting_for_output_fence() {
    let (_child, tree) = exited_empty_closure_fixture();
    let root = tree.root();
    let mut custody = Some(PtyProcessCustody::tracking(tree));
    let mut force = PtyForceTermination::new();
    let mut output = ScriptedForceOutput::new(FenceBehavior::Pending);

    drive_until(
        &mut force,
        &mut custody,
        &mut output,
        CustodyKind::GonePending,
    );

    assert_eq!(output.begin_calls, NO_OUTPUT_FENCE_CALLS);
    assert_eq!(output.poll_calls, NO_OUTPUT_FENCE_CALLS);
    assert_pending(poll_force(&mut force, &mut custody, &mut output));
    assert_variant_root(&custody, CustodyKind::GoneReported, root);
    assert_eq!(output.report_calls, EXPECTED_OPERATION_CALLS);
}

#[test]
fn exited_empty_closure_does_not_require_unused_destructive_authority() {
    let (_child, mut tree) = exited_empty_closure_fixture();
    let root = tree.root();
    tree.deny_destructive_for_test();
    let mut custody = Some(PtyProcessCustody::tracking(tree));
    let mut force = PtyForceTermination::new();
    let mut output = ScriptedForceOutput::new(FenceBehavior::Pending);

    drive_until(
        &mut force,
        &mut custody,
        &mut output,
        CustodyKind::GonePending,
    );

    assert_variant_root(&custody, CustodyKind::GonePending, root);
}

#[test]
fn stale_empty_closure_refreeze_discards_its_output_fence() {
    let (_child, tree) = ChildGuard::spawn();
    let root = tree.root();
    let mut custody = Some(PtyProcessCustody::tracking(tree));
    let mut force = PtyForceTermination::new();
    let mut output = ScriptedForceOutput::new(FenceBehavior::Pending);
    drive_to_frozen(&mut force, &mut custody, &mut output);
    assert_pending(poll_force(&mut force, &mut custody, &mut output));
    assert_eq!(output.begin_calls, FIRST_OUTPUT_FENCE_CALLS);
    match custody.as_mut().unwrap() {
        PtyProcessCustody::Frozen(tree) => tree.clear_closure_for_test(),
        _ => panic!("{EXPECTED_FROZEN_CUSTODY_DETAIL}"),
    }

    assert_pending(poll_force(&mut force, &mut custody, &mut output));

    assert_variant_root(&custody, CustodyKind::Freezing, root);
    drive_to_frozen(&mut force, &mut custody, &mut output);
    assert_pending(poll_force(&mut force, &mut custody, &mut output));
    assert_eq!(output.begin_calls, SECOND_OUTPUT_FENCE_CALLS);
}
