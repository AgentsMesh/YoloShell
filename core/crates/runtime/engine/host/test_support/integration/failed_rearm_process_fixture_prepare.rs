pub async fn prepare(home: &Path) -> Result<PreparedEvidence> {
    let host = Host::open(fixture_config(home))?;
    let manifest = fixture_manifest(home);
    host.ensure_session_namespace(manifest.terminal_id)?;
    host.write_namespace_json(&host.paths.manifest(manifest.terminal_id), &manifest)?;
    host.catalog_insert(&manifest).await?;

    let launch = host.prepare_worker_launch(&manifest, shell_command(&manifest.cwd), None, None)?;
    let transaction = ResumeTransaction::new(
        Uuid::new_v4(),
        ResumeIntent::Explicit,
        ResumeTarget::Shell {
            binding: ResumeBindingDisposition::Clear {
                operation_id: Uuid::new_v4(),
                previous: None,
                codex_authority: None,
            },
        },
        &launch,
    )?;
    let runtime = failed_runtime(&host, &transaction);
    let checkpoint = failed_checkpoint(&transaction, &runtime)?;
    host.write_namespace_json(&host.paths.runtime_record(manifest.terminal_id), &runtime)?;
    host.publish_resume_transaction(&transaction)?;
    host.publish_worker_launch(&launch)?;
    host.write_namespace_json(
        &host.paths.terminate_tombstone(manifest.terminal_id),
        &crate::model::TerminationTombstone::user_request(
            manifest.terminal_id,
            transaction.previous_runtime_epoch,
            false,
        ),
    )?;
    host.write_namespace_json(
        &host.paths.worker_startup_checkpoint(manifest.terminal_id),
        &checkpoint,
    )?;
    let evidence = PreparedEvidence {
        controller_generation: host.controller_generation,
        terminal_id: manifest.terminal_id,
        transaction,
        runtime,
        checkpoint,
    };
    verify_prepared_state(&host, &evidence)?;
    Ok(evidence)
}

fn fixture_config(home: &Path) -> HostConfig {
    let mut config = HostConfig::new(home);
    config.runtime_executable = Some(home.join(RESUME_TEST_MISSING_RUNTIME));
    config
}

fn fixture_manifest(home: &Path) -> TerminalManifest {
    let timestamp = now_ms();
    let command = shell_command(home);
    TerminalManifest {
        schema_version: SESSION_MANIFEST_SCHEMA_VERSION,
        terminal_id: Uuid::new_v4(),
        name: FIXTURE_SESSION_NAME.to_owned(),
        command: command.executable.to_string_lossy().into_owned(),
        args: command.args,
        cwd: home.to_owned(),
        cols: DEFAULT_COLS,
        rows: DEFAULT_ROWS,
        created_at_ms: timestamp,
        updated_at_ms: timestamp,
        archived: false,
        recovery_state: RecoveryState::Restoring,
    }
}

fn failed_runtime(host: &Host, transaction: &ResumeTransaction) -> RuntimeRecord {
    let launch = &transaction.launch;
    RuntimeRecord {
        schema_version: RUNTIME_RECORD_SCHEMA_VERSION,
        terminal_id: transaction.terminal_id,
        runtime_epoch: transaction.planned_runtime_epoch,
        worker_id: transaction.worker_id,
        worker_pid: RESUME_TEST_DEAD_WORKER_PID,
        worker_start_time: RESUME_TEST_PROCESS_START_TIME,
        child_pid: RESUME_TEST_DEAD_CHILD_PID,
        child_start_time: RESUME_TEST_PROCESS_START_TIME,
        boot_id: RESUME_TEST_PREVIOUS_BOOT_ID.to_owned(),
        socket_path: launch.socket_path.clone(),
        auth_token: launch.auth_token.clone(),
        controller_generation: host.controller_generation,
        started_at_ms: transaction.created_at_ms,
        command: launch.command.clone(),
        args: launch.args.clone(),
        cwd: launch.cwd.clone(),
        cols: launch.cols,
        rows: launch.rows,
        bootstrap_delivery: BootstrapDelivery::Failed,
    }
}

fn failed_checkpoint(
    transaction: &ResumeTransaction,
    runtime: &RuntimeRecord,
) -> Result<StartupCheckpointEvidence> {
    let fence = StartupLaunchFence::from_launch(&transaction.launch)?;
    Ok(StartupCheckpointEvidence {
        schema_version:
            yolo_startup_checkpoint_contract::constants::STARTUP_CHECKPOINT_SCHEMA_VERSION,
        terminal_id: runtime.terminal_id,
        runtime_epoch: runtime.runtime_epoch,
        worker_id: runtime.worker_id,
        controller_generation: transaction.launch.controller_generation,
        launch_sha256: fence.launch_sha256,
        boot_id: runtime.boot_id.clone(),
        worker: StartupProcessIdentity {
            pid: runtime.worker_pid,
            start_time: runtime.worker_start_time,
        },
        child: Some(StartupProcessIdentity {
            pid: runtime.child_pid,
            start_time: runtime.child_start_time,
        }),
        stage: StartupStage::CommandExec,
        phase: StartupPhase::Entered,
        status: StartupStatus::Active,
        successor: None,
        predecessor_runtime_sha256: None,
        pending_pty_custody_rollover: None,
    })
}

fn verify_prepared_state(host: &Host, evidence: &PreparedEvidence) -> Result<()> {
    let launch: WorkerLaunchSpec =
        host.read_namespace_json(&host.paths.worker_launch(evidence.terminal_id))?;
    let tombstone: crate::model::TerminationTombstone =
        host.read_namespace_json(&host.paths.terminate_tombstone(evidence.terminal_id))?;
    ensure!(
        host.load_resume_transaction(evidence.terminal_id)? == Some(evidence.transaction.clone())
            && launch == evidence.transaction.launch
            && tombstone.runtime_epoch == evidence.transaction.previous_runtime_epoch
            && host
                .load_durable_manifest(evidence.terminal_id)?
                .recovery_state
                == RecoveryState::Restoring
            && host.read_namespace_json::<RuntimeRecord>(
                &host.paths.runtime_record(evidence.terminal_id)
            )? == evidence.runtime
            && host.read_namespace_json::<StartupCheckpointEvidence>(
                &host.paths.worker_startup_checkpoint(evidence.terminal_id)
            )? == evidence.checkpoint
            && !evidence.socket_path().exists(),
        FIXTURE_PREPARED_STATE_MISMATCH
    );
    Ok(())
}
