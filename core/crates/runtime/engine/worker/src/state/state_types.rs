#[derive(Debug, Clone)]
pub(crate) enum WorkerEvent {
    Output(OutputChunk),
    /// The parser lost its screen history; clients must adopt a fresh snapshot.
    ScreenReset(OutputGapPayload),
    Exited(ExitRecord),
    WriterLease {
        event_type: &'static str,
        payload: serde_json::Value,
    },
    Agent {
        event_type: &'static str,
        binding: AgentBinding,
    },
}

pub(crate) struct WorkerState {
    pub(crate) paths: Paths,
    pub(crate) agent_registry: Arc<AgentRegistry>,
    pub(crate) provider_runtime: Arc<ProviderRuntimeRegistry>,
    pub(crate) namespace: WorkerNamespace,
    pub(crate) record: Mutex<RuntimeRecord>,
    pub(crate) master: Mutex<Option<Box<dyn MasterPty + Send>>>,
    pub(crate) pty_tty_path: PathBuf,
    pub(crate) writer: Mutex<Option<PtyInputWriter>>,
    pub(crate) journal: Arc<Mutex<Journal>>,
    pub(crate) terminal: Mutex<vt100::Parser<yolo_terminal_metadata::MetadataCallbacks>>,
    pub(crate) dimensions: Mutex<(u16, u16)>,
    pub(crate) snapshot_publication: Mutex<()>,
    // Last journal sequence whose bytes cannot be reconstructed into the live
    // screen parser. Zero means the parser still represents all output.
    pub(crate) screen_baseline: AtomicU64,
    pub(crate) exit: Mutex<Option<ExitRecord>>,
    pub(crate) termination_intent: Mutex<Option<TerminationIntent>>,
    pub(crate) pty_owner: Mutex<Option<pty_owner_handle::PtyOwnerHandle>>,
    pub(crate) fault_reap_observer: OnceLock<pty_fault_reaper::FaultReapStateObserver>,
    pub(crate) lease: Mutex<LeaseAuthority>,
    pub(crate) mutation_gate: Arc<tokio::sync::Mutex<()>>,
    pub(crate) secondary_custody_adoption_gate: tokio::sync::Mutex<()>,
    pub(crate) agent_generation_gate: Arc<tokio::sync::Mutex<()>>,
    pub(crate) pty_session_capability: Uuid,
    pub(crate) codex_hook_ingress: codex_hook::CodexHookIngressSupervisor,
    pub(crate) controller_generation: AtomicU64,
    pub(crate) controller_persisted_generation: AtomicU64,
    pub(crate) ready: AtomicBool,
    pub(crate) runtime_serving: AtomicBool,
    pub(crate) natural_exit_shutdown_scheduled: AtomicBool,
    pub(crate) child_waited: Arc<AtomicBool>,
    pub(crate) output_block: OutputBlock,
    pub(crate) exit_finalization_started: AtomicBool,
    pub(crate) response_publication: Mutex<response_drain::ResponsePublicationState>,
    pub(crate) response_drain_completed: Notify,
    pub(crate) reader_lifecycle: Arc<ReaderLifecycle>,
    pub(crate) pty_alias_custody: Arc<pty_runtime_alias_custody::PtyRuntimeAliasCustody>,
    pub(crate) secondary_custody: Mutex<Option<Arc<WorkerSecondaryCustodySource>>>,
    pub(crate) secondary_custody_authority:
        secondary_custody_authority::WorkerSecondaryCustodyAuthority,
    pub(crate) delivered_inputs: Mutex<InputDeliveryCache>,
    pub(crate) native_control: Mutex<Option<yolo_agent_control_runtime::ControlRuntime>>,
    pub(crate) native_control_initialized: AtomicBool,
    pub(crate) pending_interaction:
        Mutex<Option<native_control::interaction::PendingInteraction>>,
    pub(crate) agent_title: Mutex<Option<String>>,
    pub(crate) attachment_clients: Arc<Semaphore>,
    pub(crate) events: broadcast::Sender<WorkerEvent>,
    pub(crate) agent_events: broadcast::Sender<yolo_runtime_protocol::AgentEvent>,
    pub(crate) shutdown: Notify,
    #[cfg(test)]
    pub(crate) control_test_hooks: control_test_hooks::ControlTestHooks,
    #[cfg(test)]
    pub(crate) native_control_test_hooks: native_control::test_hooks::NativeControlTestHooks,
}

impl WorkerState {
    pub(crate) fn publish_agent_event(&self, event_type: &'static str, binding: AgentBinding) {
        let event = yolo_runtime_protocol::AgentEvent { event_type: event_type.into(), binding: binding.clone() };
        let _ = self.agent_events.send(event);
        let _ = self.events.send(WorkerEvent::Agent { event_type, binding });
    }
}
