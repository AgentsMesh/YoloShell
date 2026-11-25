const REPORT_FAILURE_DETAIL: &str = "injected worker producer-gone report failure";
const PROGRESS_FAILURE_DETAIL: &str = "injected worker post-report progress failure";
const EXPECTED_OPERATION_CALLS: usize = 1;

#[test]
fn report_error_preserves_gone_pending_authority() {
    let (mut child, tree) = ChildGuard::spawn();
    let root = tree.root();
    let proof = tree.root_exit_proof_for_test(root);
    let mut custody = Some(PtyProcessCustody::tracking(tree));
    let mut force = PtyForceTermination::new();
    let mut output =
        ScriptedForceOutput::new(FenceBehavior::Reached).with_report_error(REPORT_FAILURE_DETAIL);
    drive_to_gone_pending(&mut child, proof, &mut force, &mut custody, &mut output);

    let error = expect_error(poll_force(&mut force, &mut custody, &mut output));

    assert_error_contains(&error, REPORT_FAILURE_DETAIL);
    assert_eq!(output.report_calls, EXPECTED_OPERATION_CALLS);
    assert_variant_root(&custody, CustodyKind::GonePending, root);
}

#[test]
fn progress_error_after_report_preserves_gone_reported_authority() {
    let (mut child, tree) = ChildGuard::spawn();
    let root = tree.root();
    let proof = tree.root_exit_proof_for_test(root);
    let mut custody = Some(PtyProcessCustody::tracking(tree));
    let mut force = PtyForceTermination::new();
    let mut output = ScriptedForceOutput::new(FenceBehavior::Reached)
        .with_progress_error(PROGRESS_FAILURE_DETAIL);
    drive_to_gone_pending(&mut child, proof, &mut force, &mut custody, &mut output);

    assert_pending(poll_force(&mut force, &mut custody, &mut output));
    let error = expect_error(poll_force(&mut force, &mut custody, &mut output));

    assert_error_contains(&error, PROGRESS_FAILURE_DETAIL);
    assert_eq!(output.report_calls, EXPECTED_OPERATION_CALLS);
    assert_eq!(output.progress_calls, EXPECTED_OPERATION_CALLS);
    assert_variant_root(&custody, CustodyKind::GoneReported, root);
}

fn drive_to_gone_pending(
    child: &mut ChildGuard,
    proof: crate::process::UnreapedRootExit,
    force: &mut PtyForceTermination,
    custody: &mut Option<PtyProcessCustody>,
    output: &mut ScriptedForceOutput,
) {
    drive_to_killing(force, custody, output);
    custody
        .as_mut()
        .unwrap()
        .mark_root_exited_unreaped(proof)
        .unwrap();
    child.reap();
    drive_until(force, custody, output, CustodyKind::GonePending);
}
