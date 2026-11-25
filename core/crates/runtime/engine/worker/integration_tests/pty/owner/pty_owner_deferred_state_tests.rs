const DEFERRED_OWNER_RETURN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
const DEFERRED_COMPLETION_STRONG_CUSTODY_COUNT: usize = 1;
const DEFERRED_OWNER_BLOCKED_DETAIL: &str = "deferred owner did not return after custody handoff";

struct DeferredPendingRootObserver;

impl pty_exit_observer::RootExitObserver for DeferredPendingRootObserver {
    fn poll_exited_unreaped(
        &mut self,
        _authority: &process::RootWaitAuthority,
    ) -> anyhow::Result<Option<process::UnreapedRootExit>> {
        Ok(None)
    }
}

#[test]
fn deferred_natural_exit_remains_natural_after_async_finalization() {
    assert_deferred_owner_finalization(ExitReason::Natural, TerminalRuntimeStatus::OutputBlocked);
}

#[test]
fn deferred_user_termination_remains_user_termination_after_async_finalization() {
    assert_deferred_owner_finalization(
        ExitReason::UserTerminate,
        TerminalRuntimeStatus::Finalizing,
    );
}

fn assert_deferred_owner_finalization(reason: ExitReason, pending_status: TerminalRuntimeStatus) {
    let fixture = Fixture::new();
    fixture.state.reader_lifecycle.mark_drained();
    if reason == ExitReason::UserTerminate {
        fixture.state.begin_termination_for_test(reason);
    }
    let child = fixture.child.lock().unwrap().take().unwrap();
    let root = process::ProcessIdentity::capture(child.process_id().unwrap()).unwrap();
    let mut tree = process::CapturedProcessTree::capture(root).unwrap();
    tree.authorize_destructive_for_test();
    let (reservation, held) =
        pty_fault_reaper::FaultReaperReservation::reserve_held_for_test().unwrap();
    let mut ownership =
        pty_ownership::PtyOwnership::new_for_test(child, tree, reservation).unwrap();
    if reason == ExitReason::Natural {
        assert!(root.signal(libc::SIGKILL).unwrap());
        let proof = wait_for_owner_root_exit(&ownership, root);
        ownership.mark_root_exited_unreaped(proof).unwrap();
    }
    let (_commands, receiver) = std::sync::mpsc::channel();
    let settlement = crate::worker::pty_owner_handle::new_owner_settlement_for_test();
    let owner_settlement = Arc::clone(&settlement);
    let state = Arc::downgrade(&fixture.state);
    let baseline = Arc::strong_count(&fixture.state);
    let (returned, observation) = std::sync::mpsc::channel();
    let owner = std::thread::spawn(move || {
        let mut owner = ownership.into_owner_for_test(
            state,
            Box::new(DeferredPendingRootObserver),
            receiver,
            owner_settlement,
        );
        owner.finish_cleanup_deferred(termination_timing::CapabilitySettlementDeferred);
        returned.send(()).unwrap();
    });

    let held = held.wait_for_handoff();
    observation
        .recv_timeout(DEFERRED_OWNER_RETURN_TIMEOUT)
        .expect(DEFERRED_OWNER_BLOCKED_DETAIL);
    assert!(lock_recover(&fixture.state.exit).is_none());
    assert!(
        !fixture
            .state
            .paths
            .exit_record(fixture.terminal_id())
            .exists()
    );
    assert_ne!(fixture.state.termination_reason(), Some(ExitReason::Lost));
    assert_eq!(fixture.state.status().status, pending_status);
    assert_eq!(
        Arc::strong_count(&fixture.state),
        baseline + DEFERRED_COMPLETION_STRONG_CUSTODY_COUNT
    );
    assert!(matches!(
        settlement.get(),
        Some(Err(
            pty_owner_handle::PtyOwnerCommandFailure::CleanupDeferred(_)
        ))
    ));

    held.resume();
    owner.join().unwrap();
    assert!(!fixture.state.output_blocked());

    let exit = lock_recover(&fixture.state.exit).clone().unwrap();
    let durable: ExitRecord =
        read_json(&fixture.state.paths.exit_record(fixture.terminal_id())).unwrap();
    assert_eq!(exit.reason, reason);
    assert_eq!(durable.reason, reason);
    assert_eq!(Arc::strong_count(&fixture.state), baseline);
    assert!(!root.matches());
}
