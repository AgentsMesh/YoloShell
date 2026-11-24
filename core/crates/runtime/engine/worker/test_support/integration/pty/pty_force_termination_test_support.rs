#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CustodyKind {
    Tracking,
    Freezing,
    Frozen,
    Killing,
    Survivor,
    GonePending,
    GoneReported,
    MasterClosePending,
}

const EMPTY_CLOSURE_ROOT_EXIT_TIMEOUT_DETAIL: &str =
    "empty-closure root did not exit before force settlement";

fn assert_pending(result: Result<ForceTerminationPoll>) {
    match result {
        Ok(ForceTerminationPoll::Pending) => {}
        Ok(ForceTerminationPoll::Complete) => panic!("{UNEXPECTED_FORCE_COMPLETION_DETAIL}"),
        Ok(ForceTerminationPoll::Unsupported(error)) => {
            panic!("{UNEXPECTED_FORCE_ERROR_DETAIL}: {error}")
        }
        Err(error) => panic!("{UNEXPECTED_FORCE_ERROR_DETAIL}: {error:#}"),
    }
}

fn assert_error_contains(error: &anyhow::Error, detail: &str) {
    assert!(
        error
            .chain()
            .any(|cause| cause.to_string().contains(detail))
    );
}

fn expect_error(result: Result<ForceTerminationPoll>) -> anyhow::Error {
    match result {
        Err(error) => error,
        Ok(ForceTerminationPoll::Pending) => panic!("{UNEXPECTED_FORCE_ERROR_DETAIL}"),
        Ok(ForceTerminationPoll::Complete) => panic!("{UNEXPECTED_FORCE_COMPLETION_DETAIL}"),
        Ok(ForceTerminationPoll::Unsupported(error)) => {
            panic!("{UNEXPECTED_FORCE_ERROR_DETAIL}: {error}")
        }
    }
}

fn assert_variant_root(
    custody: &Option<PtyProcessCustody>,
    expected: CustodyKind,
    root: ProcessIdentity,
) {
    let (actual, actual_root) = custody_snapshot(custody);
    assert_eq!(actual, expected);
    assert_eq!(actual_root, root);
}

fn drive_to_frozen(
    force: &mut PtyForceTermination,
    custody: &mut Option<PtyProcessCustody>,
    output: &mut ScriptedForceOutput,
) {
    drive_until(force, custody, output, CustodyKind::Frozen);
}

fn drive_to_killing(
    force: &mut PtyForceTermination,
    custody: &mut Option<PtyProcessCustody>,
    output: &mut ScriptedForceOutput,
) {
    drive_until(force, custody, output, CustodyKind::Killing);
}

fn drive_until(
    force: &mut PtyForceTermination,
    custody: &mut Option<PtyProcessCustody>,
    output: &mut ScriptedForceOutput,
    expected: CustodyKind,
) {
    let deadline = Instant::now() + TRANSITION_TIMEOUT;
    while custody_snapshot(custody).0 != expected {
        assert!(
            Instant::now() < deadline,
            "{TRANSITION_TIMEOUT_DETAIL}: expected {expected:?}, got {:?}: {:#?}",
            custody_snapshot(custody).0,
            custody
                .as_ref()
                .map(PtyProcessCustody::state_snapshot_for_test)
        );
        assert_pending(poll_force(force, custody, output));
        std::thread::sleep(TRANSITION_POLL_INTERVAL);
    }
}

fn custody_snapshot(custody: &Option<PtyProcessCustody>) -> (CustodyKind, ProcessIdentity) {
    match custody.as_ref().unwrap() {
        PtyProcessCustody::Tracking(tree) => (CustodyKind::Tracking, tree.root()),
        PtyProcessCustody::Freezing(tree) => (CustodyKind::Freezing, tree.root()),
        PtyProcessCustody::Frozen(tree) => (CustodyKind::Frozen, tree.root()),
        PtyProcessCustody::Killing(tree) => (CustodyKind::Killing, tree.root()),
        PtyProcessCustody::Survivor(tree) => (CustodyKind::Survivor, tree.root()),
        PtyProcessCustody::GonePending(completion) => (CustodyKind::GonePending, completion.root()),
        PtyProcessCustody::GoneReported(completion) => {
            (CustodyKind::GoneReported, completion.root())
        }
        PtyProcessCustody::MasterClosePending(completion) => {
            (CustodyKind::MasterClosePending, completion.root())
        }
    }
}

fn exited_empty_closure_fixture() -> (ChildGuard, CapturedProcessTree) {
    let (mut child, mut tree) = ChildGuard::spawn();
    let root = tree.root();
    let proof = tree.root_exit_proof_for_test(root);
    assert!(root.signal(libc::SIGKILL).unwrap());
    let deadline = Instant::now() + TRANSITION_TIMEOUT;
    while root.matches() {
        assert!(
            Instant::now() < deadline,
            "{EMPTY_CLOSURE_ROOT_EXIT_TIMEOUT_DETAIL}"
        );
        std::thread::sleep(TRANSITION_POLL_INTERVAL);
    }
    tree.mark_root_exited_unreaped(proof).unwrap();
    child.reap();
    (child, tree)
}
