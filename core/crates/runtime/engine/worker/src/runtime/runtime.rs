pub(crate) async fn run_session(
    paths: Paths,
    terminal_id: Uuid,
    provider_runtime: Arc<ProviderRuntimeRegistry>,
    cleanup: &mut StartupCleanupScope,
) -> Result<()> {
    let instance_lock = WorkerInstanceLock::acquire(&paths, terminal_id)?;
    let namespace = WorkerNamespace::capture(&paths, terminal_id)?;
    let agent_registry = Arc::new(crate::agent::agent_registry_from_runtime(&provider_runtime));
    let PreparedWorkerStartup {
        launch,
        agent_monitor,
        worker_identity,
        mut checkpoint,
    } = prepare_worker_startup(&namespace, &paths, terminal_id, Arc::clone(&provider_runtime))?;
    checkpoint.enter(&namespace, &paths, Stage::StartupContainment)?;
    #[cfg(target_os = "linux")]
    let mut containment = prepare_startup_containment(&namespace, &paths, &launch)?;
    checkpoint.complete(&namespace, &paths, Stage::StartupContainment)?;
    checkpoint.enter(&namespace, &paths, Stage::WorkerSocket)?;
    let (listener, _socket_cleanup) =
        bind_worker_socket_retained(instance_lock, &namespace, &launch.socket_path).await?;
    checkpoint.complete(&namespace, &paths, Stage::WorkerSocket)?;
    checkpoint.enter(&namespace, &paths, Stage::AgentRuntime)?;
    let hook_executable = std::env::current_exe()?.canonicalize()?;
    let claude_hook =
        ClaudeHookServer::bind(&namespace, &paths, terminal_id, hook_executable.clone()).await?;
    let _ = yolo_qwen_hooks::remove_legacy_outbox(&paths.session_dir(terminal_id));
    let provider_hook = runtime_provider_hook::bind(&namespace, &paths, terminal_id, hook_executable.clone()).await?;
    let hook_context = NativeHookContext::new(&namespace, &paths, terminal_id, hook_executable)?;
    namespace.verify()?;
    checkpoint.complete(&namespace, &paths, Stage::AgentRuntime)?;
    checkpoint.enter(&namespace, &paths, Stage::TerminalJournal)?;
    let journal = Arc::new(Mutex::new(Journal::open_retained(
        namespace.file(&paths.journal(terminal_id))?,
        launch.runtime_epoch,
    )?));
    checkpoint.complete(&namespace, &paths, Stage::TerminalJournal)?;
    checkpoint.enter(&namespace, &paths, Stage::Pty)?;
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(startup_pty_size(&launch))?;
    let (mut pty_proof, gate_token) =
        process::PtySessionProof::capture_fresh_bootstrap_bound(pair.master.as_ref())
            .context(PTY_PROOF_CAPTURE_CONTEXT)?;
    let bootstrap_exec = pty_proof.take_bootstrap_exec_authority()?;
    pty_proof.bind_runtime(terminal_id, launch.runtime_epoch)?;
    #[cfg(target_os = "linux")]
    if let Some(lease) = containment.as_mut() {
        lease.bind_pty_proof(&pty_proof)?;
        persist_startup_containment(&namespace, &paths, lease.record())?;
    }
    let output_progress = ExclusivePtyOutput::capture(pair.master.as_ref(), Arc::clone(&journal))?;
    checkpoint.complete(&namespace, &paths, Stage::Pty)?;
    checkpoint.enter(&namespace, &paths, Stage::ChildProcess)?;
    let gate_spec = bootstrap_child_spec(&launch, gate_token)?;
    let command = bootstrap_command(&launch, &gate_spec, Some(claude_hook.environment()), Some(provider_hook.environment()), Some(&hook_context))?;
    let child = spawn_tracked_pty_root(
        pair.slave.as_ref(),
        command,
        pty_proof,
        output_progress,
        cleanup,
    )
    .context(STARTUP_CHILD_SPAWN_CONTEXT)?;
    let mut startup = StartupPtyResources::from_bootstrap_guard(pair.master, child, bootstrap_exec);
    drop(pair.slave);
    let child_identity = startup
        .child()
        .identity()
        .context(STARTUP_CHILD_IDENTITY_CONTEXT)?;
    #[cfg(target_os = "linux")]
    if let Some(containment) = containment {
        let record = startup
            .child_mut()
            .bind_linux_containment(containment, child_identity)?;
        persist_startup_containment(&namespace, &paths, &record)?;
    }
    checkpoint.complete_child(&namespace, &paths, child_identity)?;
    checkpoint.enter(&namespace, &paths, Stage::BootstrapGate)?;
    let gate = accept_bootstrap_gate(&listener, &gate_spec.expected(child_identity)).await?;
    #[cfg(target_os = "macos")]
    startup
        .child_mut()
        .install_process_capability(gate.descriptor_capability())?;
    let gate = gate.prepare().await?;
    let codex_hook_ingress = runtime_codex_ingress::start(
        &namespace,
        &paths,
        &launch,
        &hook_context.executable,
        child_identity,
        agent_registry,
    )
    .await?;
    let reader = startup.master()?.try_clone_reader()?;
    let writer = PtyInputWriter::open(startup.master()?.as_ref())?;
    checkpoint.complete(&namespace, &paths, Stage::BootstrapGate)?;
    checkpoint.enter(&namespace, &paths, Stage::RuntimeRecord)?;
    let record = pending_runtime_record(
        &launch,
        worker_identity.pid,
        worker_identity.start_time,
        child_identity,
    );
    let mut record = runtime_record_publication::publish_pending(
        &namespace,
        &paths,
        record,
        &mut startup,
        &mut checkpoint,
    )
    .await?;
    checkpoint.complete(&namespace, &paths, Stage::RuntimeRecord)?;
    checkpoint.enter(&namespace, &paths, Stage::SecondaryCustody)?;
    let custody = establish_secondary_custody(
        &listener,
        &namespace,
        &paths,
        &record,
        &mut startup,
        &mut checkpoint,
    )?;
    let (custody_source, custody_lease) = custody.await?;
    startup
        .child_mut()
        .require_host_release(custody_lease.holder())?;
    checkpoint.complete(&namespace, &paths, Stage::SecondaryCustody)?;
    checkpoint.enter(&namespace, &paths, Stage::CommandExec)?;
    let bootstrap_exec = startup.take_bootstrap_exec()?;
    let released_gate = gate.release().await?;
    startup.child_mut().commit_bootstrap_gate_release()?;
    match released_gate.confirm_exec(bootstrap_exec).await {
        Err(error) => {
            let custody = (startup, reader, writer, custody_source, custody_lease);
            return quarantine_uncertain_exec(terminal_id, error, custody).await;
        }
        Ok(BootstrapExecConfirmation::ExplicitFailure(failure)) => {
            let detail = failure.detail().to_owned();
            record.bootstrap_delivery = BootstrapDelivery::Failed;
            publish_runtime_record(&namespace, &paths, &record)?;
            startup
                .child()
                .refresh()
                .context(BOOTSTRAP_FAILURE_CONTEXT)?;
            namespace
                .remove_file(&paths.worker_launch(terminal_id))
                .context(WORKER_LAUNCH_RETIRE_CONTEXT)?;
            runtime_bootstrap_failure::rollback_custody(
                failure,
                &mut startup,
                reader,
                writer,
                custody_source,
                custody_lease,
            )
            .await?;
            bail!("{BOOTSTRAP_FAILURE_CONTEXT}: {BOOTSTRAP_EXEC_FAILURE_CONTEXT}: {detail}");
        }
        Ok(BootstrapExecConfirmation::Confirmed) => {
            startup
                .child_mut()
                .refresh()
                .context(STARTUP_EXEC_REFRESH_CONTEXT)?;
            checkpoint.complete(&namespace, &paths, Stage::CommandExec)?;
        }
    }
    let activated = runtime_bootstrap_input::activate(
        runtime_bootstrap_input::ActivationInput {
            paths,
            namespace,
            record,
            reader,
            writer,
            journal,
            launch: &launch,
            pty_session_capability: gate_token,
            secondary_custody: custody_source,
            claude_hook,
            provider_hook,
            codex_hook_ingress,
            provider_runtime,
            shell: child_identity,
        },
        &mut startup,
        &mut checkpoint,
    )
    .await?;
    serve_worker_runtime(
        listener,
        activated.state,
        agent_monitor,
        child_identity,
        custody_lease,
        activated.supervised_tasks,
    )
    .await
}
