const OWNER_GATE_CHANNEL_CAPACITY: usize = 1;
pub(super) const OWNER_THREAD_JOIN_TIMEOUT: Duration = Duration::from_secs(12);
const OWNER_GATE_REACHED_DETAIL: &str = "PTY owner did not reach the root observation gate";
const OWNER_GATE_RELEASE_DETAIL: &str = "release gated PTY root observer";
const OWNER_SETTLEMENT_DETAIL: &str = "PTY owner did not publish settlement before its deadline";
const OWNER_COMPLETION_DETAIL: &str = "PTY owner thread did not complete before its deadline";
const OWNER_THREAD_PANIC_DETAIL: &str = "gated PTY owner thread panicked";
const FAULT_REAPER_RESERVATION_MISSING_DETAIL: &str =
    "gated PTY owner fault-reaper reservation is missing";
const OWNER_INITIAL_ROOT_MISSING_DETAIL: &str =
    "gated PTY owner did not retain its exact root during initial capture";

struct GatedRootObserver {
    reached: Option<std::sync::mpsc::SyncSender<()>>,
    release: std::sync::mpsc::Receiver<()>,
    delegate: pty_exit_observer::WaitidRootExitObserver,
}

impl pty_exit_observer::RootExitObserver for GatedRootObserver {
    fn poll_exited_unreaped(
        &mut self,
        authority: &process::RootWaitAuthority,
    ) -> Result<Option<process::UnreapedRootExit>> {
        let proof = self.delegate.poll_exited_unreaped(authority)?;
        if proof.is_none() {
            return Ok(None);
        }
        self.hold(proof)
    }
}

impl GatedRootObserver {
    fn hold(
        &mut self,
        proof: Option<process::UnreapedRootExit>,
    ) -> Result<Option<process::UnreapedRootExit>> {
        if let Some(reached) = self.reached.take() {
            let _ = reached.send(());
            self.release.recv().context(OWNER_GATE_RELEASE_DETAIL)?;
        }
        Ok(proof)
    }
}

pub(super) struct GatedOwner {
    reached: Option<std::sync::mpsc::Receiver<()>>,
    release: Option<std::sync::mpsc::SyncSender<()>>,
    completed: Option<std::sync::mpsc::Receiver<()>>,
    handle: pty_owner_handle::PtyOwnerHandle,
    thread: Option<std::thread::JoinHandle<()>>,
    settlement: pty_owner_handle::OwnerSettlement,
    reaper_state: pty_fault_reaper::FaultReapStateObserver,
    host_release: GatedHostRelease,
    root: process::ProcessIdentity,
    pub(super) root_live: process::LiveProcessIdentity,
    reader_lifecycle: Arc<ReaderLifecycle>,
    cleanup_identities: Vec<(process::ProcessIdentity, process::LiveProcessIdentity)>,
}

impl GatedOwner {
    pub(super) fn handle(&self) -> pty_owner_handle::PtyOwnerHandle {
        self.handle.clone()
    }

    pub(super) fn wait_for_root_exit(&mut self, timeout: Duration) {
        let result = self.reached.take().unwrap().recv_timeout(timeout);
        assert!(
            result.is_ok(),
            "{OWNER_GATE_REACHED_DETAIL}: {result:?}; settlement: {:?}",
            self.settlement.get()
        );
    }

    fn release(&mut self) {
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
    }

    pub(super) fn wait_for_settlement(&self, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        while self.settlement.get().is_none() {
            assert!(Instant::now() < deadline, "{OWNER_SETTLEMENT_DETAIL}");
            std::thread::sleep(PROCESS_DISCOVERY_POLL_INTERVAL);
        }
        self.assert_completed_or_cleanup_deferred();
    }

    fn assert_completed_or_cleanup_deferred(&self) {
        assert!(matches!(
            self.settlement.get(),
            Some(Ok(()))
                | Some(Err(
                    pty_owner_handle::PtyOwnerCommandFailure::CleanupDeferred(_)
                ))
        ));
    }

    pub(super) fn join(&mut self) {
        self.release();
        self.completed
            .as_ref()
            .unwrap()
            .recv_timeout(OWNER_THREAD_JOIN_TIMEOUT)
            .expect(OWNER_COMPLETION_DETAIL);
        self.completed.take();
        self.thread
            .take()
            .unwrap()
            .join()
            .expect(OWNER_THREAD_PANIC_DETAIL);
        self.host_release.wait();
        self.assert_completed_or_cleanup_deferred();
    }
}

pub(super) fn start_gated_root_owner(fixture: &Fixture, capability: &FixturePtyCapability) -> GatedOwner {
    let root = fixture_root_identity(fixture);
    let (mut guard, host_release) = match fixture.startup.lock().unwrap().take() {
        Some(guard) => (guard, GatedHostRelease::respond(fixture)),
        None => {
            let (proof, output, release) = GatedHostRelease::prepare(fixture, capability, root);
            let child = fixture.child.lock().unwrap().take().unwrap();
            let mut guard = StartupChildGuard::with_reader_backed_output(child, output);
            guard.track_pty(root, proof).unwrap();
            (guard, release)
        }
    };
    let reaper_state = guard
        .fault_reaper
        .as_mut()
        .expect(FAULT_REAPER_RESERVATION_MISSING_DETAIL)
        .observe_state_for_test();
    pty_owner_handle::bind_guarded_fault_reap_observer(&guard, &fixture.state).unwrap();
    let mut ownership = guard.take_ownership().unwrap();
    let root_live = ownership.tree_mut().live_root();
    assert!(
        ownership.tree_mut().identities().contains(&root),
        "{OWNER_INITIAL_ROOT_MISSING_DETAIL}"
    );
    guard.disarm_after_transfer();

    let (reached_sender, reached) = std::sync::mpsc::sync_channel(OWNER_GATE_CHANNEL_CAPACITY);
    let (release, release_receiver) = std::sync::mpsc::sync_channel(OWNER_GATE_CHANNEL_CAPACITY);
    let (completed_sender, completed) = std::sync::mpsc::sync_channel(OWNER_GATE_CHANNEL_CAPACITY);
    let (commands, command_receiver) = std::sync::mpsc::channel();
    let settlement = crate::worker::pty_owner_handle::new_owner_settlement_for_test();
    let owner_settlement = Arc::clone(&settlement);
    let state = Arc::downgrade(&fixture.state);
    let thread = std::thread::spawn(move || {
        ownership
            .into_owner_for_test(
                state,
                Box::new(GatedRootObserver {
                    reached: Some(reached_sender),
                    release: release_receiver,
                    delegate: pty_exit_observer::WaitidRootExitObserver::new(root),
                }),
                command_receiver,
                owner_settlement,
            )
            .run();
        let _ = completed_sender.send(());
    });
    GatedOwner {
        reached: Some(reached),
        release: Some(release),
        completed: Some(completed),
        handle: pty_owner_handle::PtyOwnerHandle::for_test(commands, Arc::clone(&settlement)),
        thread: Some(thread),
        settlement,
        reaper_state,
        host_release,
        root,
        root_live,
        reader_lifecycle: Arc::clone(&fixture.state.reader_lifecycle),
        cleanup_identities: Vec::new(),
    }
}
