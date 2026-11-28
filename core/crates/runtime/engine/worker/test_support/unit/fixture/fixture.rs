struct Fixture {
    state: Arc<WorkerState>,
    child: Mutex<Option<Box<dyn portable_pty::Child + Send + Sync>>>,
    startup: Mutex<Option<StartupChildGuard>>,
    waiter: Mutex<Option<std::thread::JoinHandle<()>>>,
    _filesystem: FixtureFilesystem,
}

impl Fixture {
    fn new() -> Self {
        Self::with_command(CommandBuilder::new(FIXTURE_COMMAND))
    }

    fn with_command(command: CommandBuilder) -> Self {
        Self::with_spawn(command, ordinary_fixture_root)
    }

    fn with_event_capacity(capacity: usize) -> Self {
        Self::with_spawn_and_event_capacity(
            CommandBuilder::new(FIXTURE_COMMAND),
            ordinary_fixture_root,
            capacity,
        )
    }

    fn with_spawn(command: CommandBuilder, spawn: FixtureRootSpawn) -> Self {
        Self::with_spawn_and_event_capacity(command, spawn, FIXTURE_EVENT_CHANNEL_CAPACITY)
    }

    fn with_spawn_and_event_capacity(
        command: CommandBuilder,
        spawn: FixtureRootSpawn,
        capacity: usize,
    ) -> Self {
        Self::with_spawn_event_capacity_and_runtime(
            command,
            spawn,
            capacity,
            Arc::new(yolo_agent_providers::builtin_provider_runtime()),
        )
    }

    fn terminal_id(&self) -> Uuid {
        self.state.terminal_id()
    }

    fn lease_request(&self, holder: &str) -> LeaseRequest {
        LeaseRequest {
            terminal_id: self.terminal_id(),
            runtime_epoch: self.state.runtime_epoch(),
            holder_id: holder.into(),
            ttl_ms: Some(FIXTURE_LEASE_TTL_MS),
            generation: None,
            controller_instance_id: None,
            writer_capability: WriterCapability::generate(),
        }
    }

    fn controller(&self) -> AuthenticatedController {
        self.state.current_authenticated_controller()
    }

    fn child_pid(&self) -> u32 {
        self.child
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|child| child.process_id())
            .unwrap_or_else(|| lock_recover(&self.state.record).child_pid)
    }

    fn start_child_waiter(&self) {
        let child = self.child.lock().unwrap().take().unwrap();
        let waiter = spawn_waiter(child, Arc::clone(&self.state)).unwrap();
        *self.waiter.lock().unwrap() = Some(waiter);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        drop(lock_recover(&self.startup).take());
        let _ = crate::custody_guardian::terminate_test_processes_for_home(self.state.paths.home());
        if let Some(mut child) = self.child.lock().unwrap().take() {
            drop(lock_recover(&self.state.writer).take());
            drop(lock_recover(&self.state.master).take());
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(waiter) = self.waiter.lock().unwrap().take() {
            let _ = waiter.join();
        }
    }
}

fn fixture_runtime_output(
    fixture: &Fixture,
) -> crate::worker::pty_output_progress::ReaderBackedPtyOutput {
    crate::worker::pty_output_progress::ReaderBackedPtyOutput::observed_for_runtime(
        Arc::new(std::sync::atomic::AtomicUsize::new(usize::default())),
        Arc::clone(&fixture.state.reader_lifecycle),
        Arc::clone(&fixture.state.child_waited),
    )
}

include!("fixture_runtime_record.rs");
include!("fixture_root.rs");
