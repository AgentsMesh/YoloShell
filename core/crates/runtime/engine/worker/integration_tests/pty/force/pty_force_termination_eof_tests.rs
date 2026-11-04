const PRE_EOF_POLL_ATTEMPTS: usize = 4;
const STALE_EOF_POLL_ATTEMPTS: usize = 4;

#[test]
fn kernel_eof_wait_repeated_polls_preserve_frozen_custody() {
    let (_child, tree) = exited_empty_closure_fixture();
    let root = tree.root();
    let mut custody = Some(PtyProcessCustody::tracking(tree));
    let mut force = PtyForceTermination::requiring_empty_closure_output_for_test(
        EmptyClosureOutputRequirement::KernelEof,
    );
    let mut output = ScriptedForceOutput::new(FenceBehavior::Pending);
    output.set_progress(ProgressBehavior::Complete);
    drive_to_frozen(&mut force, &mut custody, &mut output);

    for _ in 0..PRE_EOF_POLL_ATTEMPTS {
        assert_pending(poll_force(&mut force, &mut custody, &mut output));
        assert_variant_root(&custody, CustodyKind::Frozen, root);
    }

    assert_ne!(output.eof_acquire_calls, usize::default());
}

#[test]
fn kernel_eof_for_current_closure_version_reaches_gone_pending() {
    let (_child, tree) = exited_empty_closure_fixture();
    let root = tree.root();
    let mut custody = Some(PtyProcessCustody::tracking(tree));
    let mut force = PtyForceTermination::requiring_empty_closure_output_for_test(
        EmptyClosureOutputRequirement::KernelEof,
    );
    let mut output = ScriptedForceOutput::new(FenceBehavior::Pending);
    drive_to_frozen(&mut force, &mut custody, &mut output);
    let version = frozen_closure_version(&custody);
    output.observe_kernel_eof();

    let authority = output.acquire_kernel_eof_authority(&version).unwrap();
    assert!(
        output
            .validates_kernel_eof_authority(&authority, &version)
            .unwrap()
    );
    drive_until(
        &mut force,
        &mut custody,
        &mut output,
        CustodyKind::GonePending,
    );
    assert_variant_root(&custody, CustodyKind::GonePending, root);
}

#[test]
fn kernel_eof_for_stale_closure_version_is_rejected_end_to_end() {
    let (mut child, tree) = ChildGuard::spawn();
    let root = tree.root();
    let root_exit = tree.root_exit_proof_for_test(root);
    let mut custody = Some(PtyProcessCustody::tracking(tree));
    let mut force = PtyForceTermination::requiring_empty_closure_output_for_test(
        EmptyClosureOutputRequirement::KernelEof,
    );
    let mut output = ScriptedForceOutput::new(FenceBehavior::Pending);
    drive_to_frozen(&mut force, &mut custody, &mut output);
    let stale = frozen_closure_version(&custody);
    clear_frozen_closure(&mut custody);
    assert_pending(poll_force(&mut force, &mut custody, &mut output));
    assert_variant_root(&custody, CustodyKind::Freezing, root);
    drive_to_frozen(&mut force, &mut custody, &mut output);
    exit_custodied_child(&mut child, &mut custody, root_exit);
    assert_pending(poll_force(&mut force, &mut custody, &mut output));
    assert_variant_root(&custody, CustodyKind::Freezing, root);
    drive_to_frozen(&mut force, &mut custody, &mut output);
    let current = frozen_closure_version(&custody);
    assert_ne!(stale, current);
    output.eof_authority_version_override = Some(stale);
    output.observe_kernel_eof();

    for _ in 0..STALE_EOF_POLL_ATTEMPTS {
        assert_pending(poll_force(&mut force, &mut custody, &mut output));
        assert_variant_root(&custody, CustodyKind::Frozen, root);
    }

    output.eof_authority_version_override = None;
    drive_until(
        &mut force,
        &mut custody,
        &mut output,
        CustodyKind::GonePending,
    );
}

#[test]
fn kernel_eof_tentative_observation_settles_before_publication() {
    let (_child, tree) = exited_empty_closure_fixture();
    let root = tree.root();
    let mut custody = Some(PtyProcessCustody::tracking(tree));
    let mut force = PtyForceTermination::requiring_empty_closure_output_for_test(
        EmptyClosureOutputRequirement::KernelEof,
    );
    let mut output = ScriptedForceOutput::new(FenceBehavior::Pending);
    drive_to_frozen(&mut force, &mut custody, &mut output);
    let lifecycle = std::sync::Arc::clone(&output.kernel_eof);
    let publication = lifecycle
        .begin_output_read()
        .unwrap()
        .kernel_eof_for_test()
        .await_eof_publication();

    assert_eq!(
        lifecycle.output_progress_state(),
        OutputProgressState::Pending
    );
    drive_until(
        &mut force,
        &mut custody,
        &mut output,
        CustodyKind::GonePending,
    );
    assert_variant_root(&custody, CustodyKind::GonePending, root);
    assert!(publication.is_current());
    publication.publish();
    assert_eq!(
        lifecycle.output_progress_state(),
        OutputProgressState::Complete
    );
}
