use super::*;
use crate::worker::ReaderLifecycle;
use std::sync::Arc;

const MISSING_EOF_HANDOFF_TIMEOUT: Duration = Duration::from_secs(1);

#[test]
fn zero_missing_eof_deadline_survives_fault_handoff() {
    let (mut ownership, observations, root, lifecycle) = quiescent_ownership();
    ownership.set_missing_eof_timeout_for_test(Duration::ZERO);
    drive_to_target_with_quiescent_reader(
        &mut ownership,
        PtyProcessCustodyKind::GoneReported,
        root,
        &lifecycle,
    );
    let expected = PtyProcessCustodySnapshot {
        kind: PtyProcessCustodyKind::GoneReported,
        root,
    };
    let (completion, receiver) = FaultReapCompletion::owner_channel();

    assert!(matches!(
        ownership.settle_after_fault(completion),
        FaultSettlement::Deferred(_)
    ));
    assert_eq!(
        observations
            .recv_timeout(MISSING_EOF_HANDOFF_TIMEOUT)
            .unwrap(),
        expected
    );
    let outcome = receiver
        .receive_timeout(MISSING_EOF_HANDOFF_TIMEOUT)
        .unwrap();

    assert!(lifecycle.is_cancelled());
    assert_eq!(outcome.evidence, ReapEvidence::Waited);
    assert_eq!(outcome.status.unwrap().signal(), Some(libc::SIGKILL));
    assert!(outcome.fault.is_none());
    lifecycle.finish_attempt();
}

fn quiescent_ownership() -> (
    PtyOwnership,
    std::sync::mpsc::Receiver<PtyProcessCustodySnapshot>,
    ProcessIdentity,
    Arc<ReaderLifecycle>,
) {
    let (reaper, observations) = FaultReaperReservation::reserve_observed_for_test()
        .expect(REAPER_RESERVATION_FAILURE_DETAIL);
    let child = spawn_detached(
        Path::new(SLEEP_EXECUTABLE),
        &[SLEEP_DURATION.into()],
        DetachedStandardError::Null,
    )
    .unwrap();
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let mut tree = CapturedProcessTree::capture(root).unwrap();
    tree.authorize_destructive_for_test();
    let lifecycle = Arc::new(ReaderLifecycle::default());
    assert!(lifecycle.begin_attempt());
    let output = ReaderBackedPtyOutput::observed_with_lifecycle(
        Arc::new(std::sync::atomic::AtomicUsize::default()),
        Arc::clone(&lifecycle),
    );
    let ownership =
        PtyOwnership::new_for_test_with_output(Box::new(child), tree, reaper, output).unwrap();
    (ownership, observations, root, lifecycle)
}

fn drive_to_target_with_quiescent_reader(
    ownership: &mut PtyOwnership,
    target: PtyProcessCustodyKind,
    root: ProcessIdentity,
    lifecycle: &ReaderLifecycle,
) {
    let mut force = PtyForceTermination::new();
    let mut output = ownership.output_recovery();
    drive_force_with_quiescent_reader(
        ownership,
        &mut force,
        &mut output,
        PtyProcessCustodyKind::Killing,
        lifecycle,
    );
    observe_and_mark_root_exit(ownership, root);
    drive_force_with_quiescent_reader(ownership, &mut force, &mut output, target, lifecycle);
}

fn drive_force_with_quiescent_reader(
    ownership: &mut PtyOwnership,
    force: &mut PtyForceTermination,
    output: &mut crate::worker::pty_output_progress::PtyOutputRecovery,
    target: PtyProcessCustodyKind,
    lifecycle: &ReaderLifecycle,
) {
    let deadline = Instant::now() + HANDOFF_TIMEOUT;
    while custody_snapshot(ownership).kind != target {
        assert!(Instant::now() < deadline, "{TRANSITION_TIMEOUT_DETAIL}");
        let progress = force.poll(
            ownership.process_custody_mut(),
            &mut ForceOutput::Reader(output),
        );
        assert!(
            matches!(progress, Ok(ForceTerminationPoll::Pending)),
            "{FORCE_COMPLETED_EARLY_DETAIL}"
        );
        lifecycle
            .begin_output_read()
            .unwrap()
            .mark_quiescent()
            .unwrap();
        std::thread::sleep(TRANSITION_POLL_INTERVAL);
    }
}
