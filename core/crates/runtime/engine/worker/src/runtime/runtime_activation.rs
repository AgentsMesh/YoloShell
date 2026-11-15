pub(crate) struct StagedWorkerRuntime {
    state: Arc<WorkerState>,
    owner: pty_owner_handle::StagedPtyOwner,
    reader: pty_reader_activation::PreparedPtyReader,
    writer: PtyInputWriter,
}

pub(crate) struct PreparedWorkerRuntime {
    pub(crate) state: Arc<WorkerState>,
    pub(crate) reader: Box<dyn Read + Send>,
    pub(crate) writer: PtyInputWriter,
}

pub(crate) const STAGED_MASTER_INVARIANT: &str =
    "staged worker retains its PTY master through final runtime publication";
pub(crate) const ALIAS_CLOSURE_RUNTIME_CONTEXT: &str = "capture the PTY alias closure runtime";
pub(crate) const STAGED_TTY_NAME_CONTEXT: &str = "capture the staged PTY terminal path";

pub(crate) fn prepare_worker_runtime(
    paths: Paths,
    namespace: WorkerNamespace,
    record: RuntimeRecord,
    startup: &StartupPtyResources<Box<dyn MasterPty + Send>>,
    reader: Box<dyn Read + Send>,
    writer: PtyInputWriter,
    journal: Arc<Mutex<Journal>>,
    launch: &WorkerLaunchSpec,
    pty_session_capability: Uuid,
    secondary_custody: Arc<WorkerSecondaryCustodySource>,
    codex_hook_ingress: codex_hook::CodexHookIngressSupervisor,
    provider_runtime: Arc<ProviderRuntimeRegistry>,
) -> Result<PreparedWorkerRuntime> {
    let pty_tty_path = startup
        .master()?
        .tty_name()
        .context(STAGED_TTY_NAME_CONTEXT)?;
    let alias_closure_runtime =
        tokio::runtime::Handle::try_current().context(ALIAS_CLOSURE_RUNTIME_CONTEXT)?;
    let state = create_worker_state(
        paths,
        namespace,
        record,
        pty_tty_path,
        journal,
        launch,
        pty_session_capability,
        secondary_custody,
        alias_closure_runtime,
        codex_hook_ingress,
        provider_runtime,
    );
    Ok(PreparedWorkerRuntime {
        state,
        reader,
        writer,
    })
}

impl PreparedWorkerRuntime {
    pub(crate) fn state(&self) -> &Arc<WorkerState> {
        &self.state
    }

    pub(crate) fn writer_mut(&mut self) -> &mut PtyInputWriter {
        &mut self.writer
    }

    pub(crate) fn stage(
        self,
        startup: &mut StartupPtyResources<Box<dyn MasterPty + Send>>,
    ) -> Result<StagedWorkerRuntime> {
        let pending_reader = stage_reader(self.reader, Arc::clone(&self.state))?;
        let reader = startup
            .child_mut()
            .prepare_reader_activation(pending_reader)?;
        let owner =
            pty_owner_handle::stage_guarded_owner(startup.child_mut(), Arc::clone(&self.state))?;
        Ok(StagedWorkerRuntime {
            state: self.state,
            owner,
            reader,
            writer: self.writer,
        })
    }
}

impl StagedWorkerRuntime {
    pub(crate) fn publish_and_activate(
        self,
        startup: &mut StartupPtyResources<Box<dyn MasterPty + Send>>,
        checkpoint: &mut startup_checkpoint::ActiveStartupCheckpoint,
    ) -> Result<Arc<WorkerState>> {
        let record = lock_recover(&self.state.record).clone();
        self.state.initialize_native_control(&record);
        publish_runtime_activation(&self.state.namespace, &self.state.paths, &record)?;
        let Self {
            state,
            owner,
            reader,
            writer,
        } = self;
        let checkpoint_state = Arc::clone(&state);
        activate_after_durable_checkpoint(
            checkpoint,
            &checkpoint_state.namespace,
            &checkpoint_state.paths,
            || -> Result<Arc<WorkerState>> {
                let master = startup
                    .take_master()
                    .unwrap_or_else(|_| unreachable!("{STAGED_MASTER_INVARIANT}"));
                state.install_pty_io(master, writer);
                startup.child_mut().activate_prepared_reader(reader);
                owner.activate(startup.child_mut())?;
                state.ready.store(true, Ordering::Release);
                Ok(state)
            },
        )?
    }
}

pub(crate) fn publish_runtime_activation(
    namespace: &WorkerNamespace,
    paths: &Paths,
    record: &RuntimeRecord,
) -> Result<()> {
    let guard = publication_guard(namespace, paths, record.terminal_id)?;
    guard.require_no_pending_intent()?;
    namespace
        .remove_file(&paths.worker_launch(record.terminal_id))
        .context(WORKER_LAUNCH_RETIRE_CONTEXT)?;
    publish_runtime_record_guarded(namespace, paths, record, &guard)
}

pub(crate) fn activate_after_durable_checkpoint<T, F>(
    checkpoint: &mut startup_checkpoint::ActiveStartupCheckpoint,
    namespace: &WorkerNamespace,
    paths: &Paths,
    activate: F,
) -> Result<T>
where
    F: FnOnce() -> T,
{
    checkpoint.retire_runtime_activated(namespace, paths)?;
    Ok(activate())
}
