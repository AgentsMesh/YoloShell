use crate::worker::tests::*;
const INJECTED_TRACKING_FAILURE: &str = "injected runtime process-tree refresh failure";

struct PendingRootObserver;

impl pty_exit_observer::RootExitObserver for PendingRootObserver {
    fn poll_exited_unreaped(
        &mut self,
        _authority: &process::RootWaitAuthority,
    ) -> Result<Option<process::UnreapedRootExit>> {
        Ok(None)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn runtime_tracking_failure_immediately_finalizes_as_lost() {
    let fixture = Fixture::new();
    let reader = lock_recover(&fixture.state.master)
        .as_ref()
        .unwrap()
        .try_clone_reader()
        .unwrap();
    spawn_reader(reader, Arc::clone(&fixture.state)).unwrap();
    let mut ownership = take_fault_ownership(&fixture);
    ownership
        .tree_mut()
        .fail_next_refresh_for_test(INJECTED_TRACKING_FAILURE);
    ownership.tree_mut().invalidate_events();
    let (_commands, receiver) = std::sync::mpsc::channel();
    let settlement = crate::worker::pty_owner_handle::new_owner_settlement_for_test();
    let owner_settlement = Arc::clone(&settlement);
    let state = Arc::downgrade(&fixture.state);
    let owner = std::thread::spawn(move || {
        ownership
            .into_owner_for_test(
                state,
                Box::new(PendingRootObserver),
                receiver,
                owner_settlement,
            )
            .run();
    });

    wait_for_runtime_status(&fixture.state, TerminalRuntimeStatus::Exited, OWNER_FAULT_TIMEOUT).await;

    assert_eq!(
        lock_recover(&fixture.state.exit).as_ref().unwrap().reason,
        ExitReason::Lost
    );
    assert!(fixture.state.child_waited.load(Ordering::Acquire));
    assert!(fixture.state.reader_lifecycle.producers_are_gone());
    assert!(lock_recover(&fixture.state.reader_lifecycle.progress).is_drained());
    let error = settlement.get().unwrap().as_ref().unwrap_err();
    assert!(error.contains(INJECTED_TRACKING_FAILURE));
    owner.join().unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn accepted_force_survives_tracking_failure_before_force_poll() {
    let fixture = Fixture::new();
    let reader = lock_recover(&fixture.state.master)
        .as_ref()
        .unwrap()
        .try_clone_reader()
        .unwrap();
    spawn_reader(reader, Arc::clone(&fixture.state)).unwrap();
    let mut ownership = take_fault_ownership(&fixture);
    ownership
        .tree_mut()
        .fail_next_refresh_for_test(INJECTED_TRACKING_FAILURE);
    ownership.tree_mut().invalidate_events();
    let (commands, receiver) = std::sync::mpsc::channel();
    let (response, completion) = tokio::sync::oneshot::channel();
    commands
        .send(pty_owner_handle::PtyOwnerCommand::Terminate {
            force: true,
            response,
        })
        .unwrap();
    let settlement = crate::worker::pty_owner_handle::new_owner_settlement_for_test();
    let owner_settlement = Arc::clone(&settlement);
    let state = Arc::downgrade(&fixture.state);
    let owner = std::thread::spawn(move || {
        ownership
            .into_owner_for_test(
                state,
                Box::new(PendingRootObserver),
                receiver,
                owner_settlement,
            )
            .run();
    });

    let completion = tokio::time::timeout(OWNER_FAULT_TIMEOUT, completion)
        .await
        .unwrap()
        .unwrap();
    assert!(completion.unwrap_err().contains(INJECTED_TRACKING_FAILURE));
    wait_for_runtime_status(&fixture.state, TerminalRuntimeStatus::Exited, OWNER_FAULT_TIMEOUT).await;

    let exit = lock_recover(&fixture.state.exit);
    let exit = exit.as_ref().unwrap();
    assert_eq!(exit.reason, ExitReason::Lost);
    assert_eq!(exit.signal, Some(libc::SIGKILL));
    assert!(fixture.state.child_waited.load(Ordering::Acquire));
    assert!(settlement.get().unwrap().is_err());
    owner.join().unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disconnected_command_channel_cannot_leave_a_running_owner() {
    let fixture = Fixture::new();
    let reader = lock_recover(&fixture.state.master)
        .as_ref()
        .unwrap()
        .try_clone_reader()
        .unwrap();
    spawn_reader(reader, Arc::clone(&fixture.state)).unwrap();
    let ownership = take_fault_ownership(&fixture);
    let (commands, receiver) = std::sync::mpsc::channel();
    drop(commands);
    let settlement = crate::worker::pty_owner_handle::new_owner_settlement_for_test();
    let owner_settlement = Arc::clone(&settlement);
    let state = Arc::downgrade(&fixture.state);
    let owner = std::thread::spawn(move || {
        ownership
            .into_owner_for_test(
                state,
                Box::new(PendingRootObserver),
                receiver,
                owner_settlement,
            )
            .run();
    });

    wait_for_runtime_status(&fixture.state, TerminalRuntimeStatus::Exited, OWNER_FAULT_TIMEOUT).await;

    assert!(fixture.state.child_waited.load(Ordering::Acquire));
    assert_eq!(
        lock_recover(&fixture.state.exit).as_ref().unwrap().reason,
        ExitReason::Lost
    );
    assert!(fixture.state.reader_lifecycle.producers_are_gone());
    assert!(lock_recover(&fixture.state.reader_lifecycle.progress).is_drained());
    assert!(settlement.get().unwrap().is_err());
    owner.join().unwrap();
}

#[test]
fn installed_owner_does_not_retain_its_worker_state_or_child() {
    let fixture = Fixture::new();
    let child = fixture.child.lock().unwrap().take().unwrap();
    let record = lock_recover(&fixture.state.record);
    let root = process::ProcessIdentity {
        pid: record.child_pid,
        start_time: record.child_start_time,
    };
    drop(record);
    let mut guard = StartupChildGuard::new(child);
    guard.install_reader_backed_output_for_test();
    guard.track(root).unwrap();
    pty_owner_handle::spawn_guarded_owner(&mut guard, Arc::clone(&fixture.state)).unwrap();
    let state = Arc::downgrade(&fixture.state);

    drop(fixture);

    let deadline = Instant::now() + OWNER_FAULT_TIMEOUT;
    while root.matches() && Instant::now() < deadline {
        std::thread::sleep(STARTUP_CHILD_EXIT_POLL_INTERVAL);
    }
    assert!(state.upgrade().is_none());
    assert!(!root.matches());
}
