fn pending_kernel_eof_fixture() -> (
    PtyOwnership,
    crate::worker::pty_output_progress::PtyOutputRecovery,
    HeldFaultReaper,
    ScriptedForceOutput,
    ProcessClosureVersion,
) {
    let reader_fence = OutputFence::Reader(OutputQuiescenceFence::AwaitingQueue {
        lifecycle: std::sync::Arc::new(crate::worker::ReaderLifecycle::default()),
    });
    let (ownership, held, scripted, version) =
        kernel_eof_ownership_fixture(FenceBehavior::Pending, reader_fence);
    let output = ownership.output_recovery();
    (ownership, output, held, scripted, version)
}

fn pending_scripted_kernel_eof_fixture(
    behavior: FenceBehavior,
) -> (
    PtyOwnership,
    HeldFaultReaper,
    ScriptedForceOutput,
    ProcessClosureVersion,
) {
    kernel_eof_ownership_fixture(behavior, OutputFence::Scripted)
}

fn kernel_eof_ownership_fixture(
    behavior: FenceBehavior,
    fence: OutputFence,
) -> (
    PtyOwnership,
    HeldFaultReaper,
    ScriptedForceOutput,
    ProcessClosureVersion,
) {
    let (reaper, held) = FaultReaperReservation::reserve_held_for_test().unwrap();
    let child = spawn_detached(
        Path::new(SLEEP_EXECUTABLE),
        &[SLEEP_DURATION.into()],
        DetachedStandardError::Null,
    )
    .unwrap();
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let mut tree = CapturedProcessTree::capture(root).unwrap();
    tree.authorize_destructive_for_test();
    let mut ownership = PtyOwnership::new_for_test_with_output(
        Box::new(child),
        tree,
        reaper,
        ReaderBackedPtyOutput::for_test(),
    )
    .unwrap();
    let mut scripted = ScriptedForceOutput::new(behavior);
    drive_ownership_to_frozen(&mut ownership, &mut scripted);
    let version = frozen_version(&mut ownership);
    scripted.observe_kernel_eof();
    let mut authority = scripted.acquire_kernel_eof_authority(&version).unwrap();
    let permit = authority.take_permit().unwrap();
    install_kernel_eof_commit(&mut ownership, permit);
    ownership.force_mut_for_test().output_fence = Some(super::VersionedOutputFence {
        version: version.clone(),
        state: fence,
        kernel_eof: Some(authority),
    });
    (ownership, held, scripted, version)
}

fn drive_ownership_to_frozen(ownership: &mut PtyOwnership, output: &mut ScriptedForceOutput) {
    let deadline = Instant::now() + TRANSITION_TIMEOUT;
    while ownership
        .process_custody_mut()
        .as_ref()
        .unwrap()
        .snapshot_for_test()
        .kind
        != PtyProcessCustodyKind::Frozen
    {
        assert!(Instant::now() < deadline, "{TRANSITION_TIMEOUT_DETAIL}");
        assert_pending(ownership.poll_force(&mut ForceOutput::Scripted(output)));
        std::thread::sleep(TRANSITION_POLL_INTERVAL);
    }
}

fn frozen_version(ownership: &mut PtyOwnership) -> ProcessClosureVersion {
    match ownership.process_custody_mut().as_ref().unwrap() {
        PtyProcessCustody::Frozen(tree) => tree.closure_version(),
        _ => panic!("{FORCE_HANDOFF_FROZEN_DETAIL}"),
    }
}

fn install_kernel_eof_commit(
    ownership: &mut PtyOwnership,
    permit: crate::process::kernel_eof_authority::KernelEofPermit,
) {
    match ownership.process_custody_mut().as_mut().unwrap() {
        PtyProcessCustody::Frozen(tree) => {
            tree.install_kernel_eof_authority_for_test(permit);
            tree.force_kernel_eof_closure_stable_for_test();
        }
        _ => panic!("{FORCE_HANDOFF_FROZEN_DETAIL}"),
    }
}

fn handoff_and_assert(
    mut ownership: PtyOwnership,
    held: HeldFaultReaper,
    eof_output: &mut ScriptedForceOutput,
    version: ProcessClosureVersion,
) {
    let (completion, receiver) = FaultReapCompletion::owner_channel();
    assert!(
        matches!(
            ownership.settle_after_fault(completion),
            FaultSettlement::Deferred(_)
        ),
        "{FORCE_HANDOFF_DEFERRED_DETAIL}"
    );
    let mut handoff = held.wait_for_handoff();
    handoff.with_force_transaction_for_test(|force, custody| {
        assert_force_transaction(force, custody, eof_output, &version);
        prepare_for_cleanup(force, custody);
    });
    handoff.resume();
    let outcome = receiver
        .receive_timeout(HANDOFF_COMPLETION_TIMEOUT)
        .unwrap();
    assert_eq!(outcome.evidence, ReapEvidence::Waited);
    assert!(outcome.status.is_some());
    assert!(outcome.fault.is_none());
}

fn assert_force_transaction(
    force: &PtyForceTermination,
    custody: &mut Option<PtyProcessCustody>,
    output: &mut ScriptedForceOutput,
    version: &ProcessClosureVersion,
) {
    assert_eq!(
        custody.as_ref().unwrap().snapshot_for_test().kind,
        PtyProcessCustodyKind::Frozen,
        "{FORCE_HANDOFF_FROZEN_DETAIL}"
    );
    let fence = force
        .output_fence
        .as_ref()
        .expect(FORCE_HANDOFF_FENCE_DETAIL);
    assert_eq!(&fence.version, version);
    assert!(matches!(
        fence.state,
        OutputFence::Reader(
            OutputQuiescenceFence::AwaitingQueue { .. }
                | OutputQuiescenceFence::AwaitingReader { .. }
        )
    ));
    let authority = fence
        .kernel_eof
        .as_ref()
        .expect(FORCE_HANDOFF_KERNEL_EOF_DETAIL);
    assert!(
        output
            .validates_kernel_eof_authority(authority, version)
            .unwrap()
    );
}

fn prepare_for_cleanup(force: &mut PtyForceTermination, custody: &mut Option<PtyProcessCustody>) {
    match custody.as_mut().unwrap() {
        PtyProcessCustody::Frozen(tree) => tree.downgrade_kill_basis_to_ordinary_for_test(),
        _ => panic!("{FORCE_HANDOFF_FROZEN_DETAIL}"),
    }
    *force = PtyForceTermination::new();
}
