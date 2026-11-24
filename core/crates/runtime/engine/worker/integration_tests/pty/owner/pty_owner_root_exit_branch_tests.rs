const OWNER_ROOT_EXIT_PROOF_TIMEOUT: Duration = Duration::from_secs(1);
const OWNER_RUN_COMPLETION_TIMEOUT: Duration = Duration::from_secs(2);
const OWNER_RUN_STALLED_DETAIL: &str = "PTY owner did not complete after retired-session root exit";

struct ProvenRootExitObserver(Option<process::UnreapedRootExit>);

impl pty_exit_observer::RootExitObserver for ProvenRootExitObserver {
    fn poll_exited_unreaped(
        &mut self,
        _authority: &process::RootWaitAuthority,
    ) -> Result<Option<process::UnreapedRootExit>> {
        Ok(self.0.take())
    }
}

#[test]
fn retired_session_root_exit_reaps_without_restarting_cleanup() {
    let fixture = Fixture::new();
    let reader = lock_recover(&fixture.state.master)
        .as_ref()
        .unwrap()
        .try_clone_reader()
        .unwrap();
    spawn_reader(reader, Arc::clone(&fixture.state)).unwrap();
    let mut ownership = take_fault_ownership(&fixture);
    ownership.tree_mut().retire_session_for_test();
    let root = fixture.state.pty_root_identity();
    assert!(root.signal(libc::SIGKILL).unwrap());
    let proof = wait_for_owner_root_exit(&ownership, root);
    fixture
        .state
        .reader_lifecycle
        .mark_producers_gone_for_test();
    let (_commands, receiver) = std::sync::mpsc::channel();
    let settlement = crate::worker::pty_owner_handle::new_owner_settlement_for_test();
    let owner = ownership.into_owner_for_test(
        Arc::downgrade(&fixture.state),
        Box::new(ProvenRootExitObserver(Some(proof))),
        receiver,
        Arc::clone(&settlement),
    );

    run_owner_bounded(owner, &fixture.state);

    assert!(settlement.get().unwrap().is_ok());
    assert!(fixture.state.child_waited.load(Ordering::Acquire));
    assert_eq!(
        lock_recover(&fixture.state.exit).as_ref().unwrap().reason,
        ExitReason::Natural
    );
}

fn run_owner_bounded(owner: pty_owner::PtyOwner, state: &Arc<WorkerState>) {
    let (completed, completion) = std::sync::mpsc::sync_channel(1);
    let owner_thread = std::thread::spawn(move || {
        owner.run();
        let _ = completed.send(());
    });
    if completion
        .recv_timeout(OWNER_RUN_COMPLETION_TIMEOUT)
        .is_err()
    {
        state.reader_lifecycle.cancel();
        owner_thread.join().unwrap();
        panic!("{OWNER_RUN_STALLED_DETAIL}");
    }
    owner_thread.join().unwrap();
}

#[test]
fn missing_eof_after_root_exit_marks_live_worker_state_lost() {
    let fixture = Fixture::new();
    let lifecycle = Arc::clone(&fixture.state.reader_lifecycle);
    let (mut ownership, output_calls) = take_stalled_output_ownership(&fixture, &lifecycle);
    ownership.set_missing_eof_timeout_for_test(Duration::ZERO);
    let root = fixture.state.pty_root_identity();
    assert!(root.signal(libc::SIGKILL).unwrap());
    let proof = wait_for_owner_root_exit(&ownership, root);
    let (_commands, receiver) = std::sync::mpsc::channel();
    let settlement = crate::worker::pty_owner_handle::new_owner_settlement_for_test();
    let owner = ownership.into_owner_for_test(
        Arc::downgrade(&fixture.state),
        Box::new(ProvenRootExitObserver(Some(proof))),
        receiver,
        Arc::clone(&settlement),
    );

    owner.run();

    assert!(settlement.get().unwrap().is_ok());
    assert!(lifecycle.is_cancelled());
    assert!(output_calls.load(Ordering::Acquire) > usize::default());
    assert!(fixture.state.output_blocked());
    assert_eq!(fixture.state.termination_reason(), Some(ExitReason::Lost));
    assert!(fixture.state.child_waited.load(Ordering::Acquire));
}

#[test]
fn missing_eof_cleanup_reaps_after_worker_state_is_released() {
    let fixture = Fixture::new();
    let lifecycle = Arc::clone(&fixture.state.reader_lifecycle);
    let (mut ownership, output_calls) = take_stalled_output_ownership(&fixture, &lifecycle);
    ownership.set_missing_eof_timeout_for_test(Duration::ZERO);
    let root = fixture.state.pty_root_identity();
    assert!(root.signal(libc::SIGKILL).unwrap());
    let proof = wait_for_owner_root_exit(&ownership, root);
    let state = Arc::downgrade(&fixture.state);
    let (_commands, receiver) = std::sync::mpsc::channel();
    let settlement = crate::worker::pty_owner_handle::new_owner_settlement_for_test();
    let owner = ownership.into_owner_for_test(
        state.clone(),
        Box::new(ProvenRootExitObserver(Some(proof))),
        receiver,
        Arc::clone(&settlement),
    );
    drop(fixture);

    owner.run();

    assert!(settlement.get().unwrap().is_ok());
    assert!(state.upgrade().is_none());
    assert!(lifecycle.is_cancelled());
    assert!(output_calls.load(Ordering::Acquire) > usize::default());
    assert!(!root.matches());
}

fn take_stalled_output_ownership(
    fixture: &Fixture,
    lifecycle: &Arc<ReaderLifecycle>,
) -> (
    pty_ownership::PtyOwnership,
    Arc<std::sync::atomic::AtomicUsize>,
) {
    let child = fixture.child.lock().unwrap().take().unwrap();
    let root = fixture.state.pty_root_identity();
    let output_calls = Arc::new(std::sync::atomic::AtomicUsize::default());
    let output = crate::worker::pty_output_progress::ReaderBackedPtyOutput::observed_with_lifecycle(
        Arc::clone(&output_calls),
        Arc::clone(lifecycle),
    );
    let mut guard = StartupChildGuard::with_reader_backed_output(child, output);
    guard.track(root).unwrap();
    let ownership = guard.take_ownership().unwrap();
    guard.disarm_after_transfer();
    (ownership, output_calls)
}

fn wait_for_owner_root_exit(
    ownership: &pty_ownership::PtyOwnership,
    root: process::ProcessIdentity,
) -> process::UnreapedRootExit {
    let deadline = Instant::now() + OWNER_ROOT_EXIT_PROOF_TIMEOUT;
    let mut observer = pty_exit_observer::WaitidRootExitObserver::new(root);
    loop {
        if let Some(proof) = ownership.poll_root_exit(&mut observer).unwrap() {
            return proof;
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
}
