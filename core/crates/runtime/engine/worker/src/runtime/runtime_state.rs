pub(crate) fn create_worker_state(
    paths: Paths,
    namespace: WorkerNamespace,
    record: RuntimeRecord,
    pty_tty_path: PathBuf,
    journal: Arc<Mutex<Journal>>,
    launch: &WorkerLaunchSpec,
    pty_session_capability: Uuid,
    secondary_custody: Arc<WorkerSecondaryCustodySource>,
    alias_closure_runtime: tokio::runtime::Handle,
    codex_hook_ingress: codex_hook::CodexHookIngressSupervisor,
    provider_runtime: Arc<ProviderRuntimeRegistry>,
) -> Arc<WorkerState> {
    let cols = normalize_terminal_columns(launch.cols);
    let rows = normalize_terminal_rows(launch.rows);
    let mut record = record;
    record.cols = cols;
    record.rows = rows;
    let (events, _) = broadcast::channel(WORKER_EVENT_CHANNEL_CAPACITY);
    let (agent_events, _) = broadcast::channel(WORKER_EVENT_CHANNEL_CAPACITY);
    let pty_alias_custody = Arc::new(pty_runtime_alias_custody::PtyRuntimeAliasCustody::new(
        alias_closure_runtime,
    ));
    let agent_registry = Arc::new(crate::agent::agent_registry_from_runtime(&provider_runtime));
    let state = Arc::new(WorkerState {
        paths,
        agent_registry,
        provider_runtime,
        agent_events,
        namespace,
        record: Mutex::new(record),
        master: Mutex::new(None),
        pty_tty_path,
        writer: Mutex::new(None),
        journal,
        terminal: Mutex::new(new_terminal_parser(cols, rows)),
        dimensions: Mutex::new((cols, rows)),
        snapshot_publication: Mutex::new(()),
        screen_baseline: AtomicU64::new(0),
        exit: Mutex::new(None),
        termination_intent: Mutex::new(None),
        pty_owner: Mutex::new(None),
        fault_reap_observer: OnceLock::new(),
        lease: Mutex::new(LeaseAuthority::default()),
        mutation_gate: Arc::new(tokio::sync::Mutex::new(())),
        secondary_custody_adoption_gate: tokio::sync::Mutex::new(()),
        agent_generation_gate: Arc::new(tokio::sync::Mutex::new(())),
        pty_session_capability,
        codex_hook_ingress,
        controller_generation: AtomicU64::new(launch.controller_generation),
        controller_persisted_generation: AtomicU64::new(launch.controller_generation),
        ready: AtomicBool::new(false),
        runtime_serving: AtomicBool::new(false),
        natural_exit_shutdown_scheduled: AtomicBool::new(false),
        child_waited: Arc::new(AtomicBool::new(false)),
        output_block: OutputBlock::default(),
        exit_finalization_started: AtomicBool::new(false),
        response_publication: Mutex::new(response_drain::ResponsePublicationState::default()),
        response_drain_completed: Notify::new(),
        reader_lifecycle: Arc::new(ReaderLifecycle::default()),
        pty_alias_custody: Arc::clone(&pty_alias_custody),
        secondary_custody: Mutex::new(Some(secondary_custody)),
        secondary_custody_authority:
            secondary_custody_authority::WorkerSecondaryCustodyAuthority::default(),
        delivered_inputs: Mutex::new(InputDeliveryCache::default()),
        native_control: Mutex::new(None),
        native_control_initialized: AtomicBool::new(false),
        pending_interaction: Mutex::new(None),
        agent_title: Mutex::new(None),
        attachment_clients: Arc::new(Semaphore::new(MAX_WORKER_ATTACHMENTS)),
        events,
        shutdown: Notify::new(),
        #[cfg(test)]
        control_test_hooks: control_test_hooks::ControlTestHooks::default(),
        #[cfg(test)]
        native_control_test_hooks: native_control::test_hooks::NativeControlTestHooks::default(),
    });
    pty_alias_custody.bind(&state);
    state
}

impl WorkerState {
    pub(crate) fn install_pty_io(&self, master: Box<dyn MasterPty + Send>, writer: PtyInputWriter) {
        let previous_master = lock_recover(&self.master).replace(master);
        let previous_writer = lock_recover(&self.writer).replace(writer);
        assert!(previous_master.is_none() && previous_writer.is_none());
    }
}

pub(crate) fn new_terminal_parser(
    cols: u16,
    rows: u16,
) -> vt100::Parser<yolo_terminal_metadata::MetadataCallbacks> {
    vt100::Parser::new_with_callbacks(
        normalize_terminal_rows(rows),
        normalize_terminal_columns(cols),
        TERMINAL_SCROLLBACK_ROWS,
        yolo_terminal_metadata::MetadataCallbacks::default(),
    )
}
