const EOF_COMMIT_PREPARED_GONE_REQUIRED_DETAIL: &str =
    "kernel EOF did not prepare a Gone transition";

fn prepare_kernel_eof_gone_ready(
    force: &mut PtyForceTermination,
    custody: &mut Option<PtyProcessCustody>,
    output: &mut ScriptedForceOutput,
) -> ForceReady {
    let ready = prepare_force_without_commit(force, custody, output);
    assert!(
        matches!(ready, ForceReady::FrozenGone),
        "{EOF_COMMIT_PREPARED_GONE_REQUIRED_DETAIL}"
    );
    ready
}

fn frozen_closure_version(custody: &Option<PtyProcessCustody>) -> ProcessClosureVersion {
    match custody.as_ref() {
        Some(PtyProcessCustody::Frozen(tree)) => tree.closure_version(),
        _ => panic!("{EXPECTED_FROZEN_CUSTODY_DETAIL}"),
    }
}

fn prepare_force_without_commit(
    force: &mut PtyForceTermination,
    custody: &mut Option<PtyProcessCustody>,
    output: &mut ScriptedForceOutput,
) -> ForceReady {
    force
        .prepare(
            custody.as_mut().unwrap(),
            &mut ForceOutput::Scripted(output),
        )
        .unwrap()
}

fn commit_prepared_force(
    force: &mut PtyForceTermination,
    custody: &mut Option<PtyProcessCustody>,
    output: &mut ScriptedForceOutput,
    ready: ForceReady,
) -> Result<ForceTerminationPoll> {
    force.commit(custody, &mut ForceOutput::Scripted(output), ready)
}

fn frozen_tree_mut(custody: &mut Option<PtyProcessCustody>) -> &mut FrozenProcessTree {
    match custody.as_mut() {
        Some(PtyProcessCustody::Frozen(tree)) => tree,
        _ => panic!("{EXPECTED_FROZEN_CUSTODY_DETAIL}"),
    }
}

fn clear_frozen_closure(custody: &mut Option<PtyProcessCustody>) {
    match custody.as_mut() {
        Some(PtyProcessCustody::Frozen(tree)) => tree.clear_closure_for_test(),
        _ => panic!("{EXPECTED_FROZEN_CUSTODY_DETAIL}"),
    }
}

fn exit_custodied_child(
    child: &mut ChildGuard,
    custody: &mut Option<PtyProcessCustody>,
    proof: UnreapedRootExit,
) {
    assert!(child.root.signal(libc::SIGKILL).unwrap());
    let deadline = Instant::now() + TRANSITION_TIMEOUT;
    while child.root.matches() {
        assert!(
            Instant::now() < deadline,
            "{EMPTY_CLOSURE_ROOT_EXIT_TIMEOUT_DETAIL}"
        );
        std::thread::sleep(TRANSITION_POLL_INTERVAL);
    }
    custody
        .as_mut()
        .unwrap()
        .mark_root_exited_unreaped(proof)
        .unwrap();
    child.reap();
}

fn deny_frozen_destructive_authority(custody: &mut Option<PtyProcessCustody>) {
    match custody.as_mut() {
        Some(PtyProcessCustody::Frozen(tree)) => tree.deny_destructive_for_test(),
        _ => panic!("{EXPECTED_FROZEN_CUSTODY_DETAIL}"),
    }
}
