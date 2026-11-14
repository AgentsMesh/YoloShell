impl Fixture {
    fn with_provider_runtime(
        provider_runtime: yolo_agent_providers::ProviderRuntimeRegistry,
    ) -> Self {
        Self::with_spawn_event_capacity_and_runtime(
            CommandBuilder::new(FIXTURE_COMMAND),
            ordinary_fixture_root,
            FIXTURE_EVENT_CHANNEL_CAPACITY,
            Arc::new(provider_runtime),
        )
    }

    fn with_spawn_event_capacity_and_runtime(
        command: CommandBuilder,
        spawn: FixtureRootSpawn,
        capacity: usize,
        provider_runtime: Arc<yolo_agent_providers::ProviderRuntimeRegistry>,
    ) -> Self {
        let filesystem = FixtureFilesystem::new();
        let paths = filesystem.paths();
        let terminal_id = Uuid::new_v4();
        paths.ensure_session(terminal_id).unwrap();
        let runtime_epoch = Uuid::new_v4();
        let journal = Arc::new(Mutex::new(
            Journal::open(paths.journal(terminal_id), runtime_epoch).unwrap(),
        ));
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: FIXTURE_TERMINAL_ROWS,
                cols: FIXTURE_TERMINAL_COLUMNS,
                pixel_width: FIXTURE_PIXEL_DIMENSION,
                pixel_height: FIXTURE_PIXEL_DIMENSION,
            })
            .unwrap();
        let root = spawn(
            &pair,
            command,
            &paths,
            terminal_id,
            runtime_epoch,
            Arc::clone(&journal),
        );
        drop(pair.slave);
        let child_pid = root.identity.pid;
        let pty_tty_path = pair.master.tty_name().unwrap();
        let writer = PtyInputWriter::open(pair.master.as_ref()).unwrap();
        let record = runtime_record(&paths, terminal_id, runtime_epoch, child_pid);
        filesystem.write_manifest(&record);
        crate::paths::atomic_write_json(&paths.runtime_record(terminal_id), &record).unwrap();
        let namespace = WorkerNamespace::capture(&paths, terminal_id).unwrap();
        let (events, _) = broadcast::channel(capacity);
        let (agent_events, _) = broadcast::channel(capacity);
        let pty_alias_custody =
            Arc::new(pty_runtime_alias_custody::PtyRuntimeAliasCustody::new_for_test());
        let agent_registry = Arc::new(crate::agent::agent_registry_from_runtime(&provider_runtime));
        let state = Arc::new(WorkerState {
            paths,
            namespace,
            record: Mutex::new(record),
            master: Mutex::new(Some(pair.master)),
            pty_tty_path,
            writer: Mutex::new(Some(writer)),
            journal,
            terminal: Mutex::new(vt100::Parser::new_with_callbacks(
                FIXTURE_TERMINAL_ROWS,
                FIXTURE_TERMINAL_COLUMNS,
                TERMINAL_SCROLLBACK_ROWS,
                yolo_terminal_metadata::MetadataCallbacks::default(),
            )),
            dimensions: Mutex::new((FIXTURE_TERMINAL_COLUMNS, FIXTURE_TERMINAL_ROWS)),
            snapshot_publication: Mutex::new(()),
            screen_baseline: AtomicU64::new(0),
            exit: Mutex::new(None),
            termination_intent: Mutex::new(None),
            pty_owner: Mutex::new(None),
            fault_reap_observer: OnceLock::new(),
            lease: Mutex::new(LeaseAuthority::default()),
            mutation_gate: Arc::new(tokio::sync::Mutex::new(())),
            secondary_custody_adoption_gate: tokio::sync::Mutex::new(()),
            agent_generation_gate: Arc::new(tokio::sync::Mutex::new(())), agent_registry, provider_runtime,
            pty_session_capability: Uuid::new_v4(),
            codex_hook_ingress: codex_hook::CodexHookIngressSupervisor::new(),
            controller_generation: AtomicU64::new(FIXTURE_CONTROLLER_GENERATION),
            controller_persisted_generation: AtomicU64::new(FIXTURE_CONTROLLER_GENERATION),
            ready: AtomicBool::new(true),
            runtime_serving: AtomicBool::new(false),
            natural_exit_shutdown_scheduled: AtomicBool::new(false),
            child_waited: Arc::new(AtomicBool::new(false)),
            output_block: OutputBlock::default(),
            exit_finalization_started: AtomicBool::new(false),
            response_publication: Mutex::new(response_drain::ResponsePublicationState::default()),
            response_drain_completed: Notify::new(),
            reader_lifecycle: Arc::new(ReaderLifecycle::default()),
            pty_alias_custody: Arc::clone(&pty_alias_custody),
            secondary_custody: Mutex::new(None),
            secondary_custody_authority: secondary_custody_authority::WorkerSecondaryCustodyAuthority::ready_for_test(FIXTURE_CONTROLLER_GENERATION),
            delivered_inputs: Mutex::new(InputDeliveryCache::default()),
            native_control: Mutex::new(None),
            native_control_initialized: AtomicBool::new(false),
            pending_interaction: Mutex::new(None),
            agent_title: Mutex::new(None),
            attachment_clients: Arc::new(Semaphore::new(MAX_WORKER_ATTACHMENTS)),
            events,
            agent_events,
            shutdown: Notify::new(),
            control_test_hooks: control_test_hooks::ControlTestHooks::default(),
            native_control_test_hooks: native_control::test_hooks::NativeControlTestHooks::default(
            ),
        });
        pty_alias_custody.bind(&state);
        Self {
            state,
            child: Mutex::new(root.child),
            startup: Mutex::new(root.startup),
            waiter: Mutex::new(None),
            _filesystem: filesystem,
        }
    }
}
