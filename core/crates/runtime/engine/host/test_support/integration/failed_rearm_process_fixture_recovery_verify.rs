async fn verify_recovered_state(
    host: &Host,
    boundary: Boundary,
    evidence: &PreparedEvidence,
    crash: &CrashSnapshot,
) -> Result<()> {
    let transaction: ResumeTransaction =
        read_json(&host.paths.resume_transaction(evidence.terminal_id))?;
    if replacement_published(boundary) {
        ensure!(transaction == crash.transaction, FIXTURE_RECOVERY_MISMATCH);
    }
    let launch: WorkerLaunchSpec = read_json(&host.paths.worker_launch(evidence.terminal_id))?;
    let checkpoint: StartupCheckpointEvidence =
        read_json(&host.paths.worker_startup_checkpoint(evidence.terminal_id))?;
    let successor = StartupLaunchFence::from_launch(&transaction.launch)?;
    let expected_generation = if replacement_published(boundary) {
        transaction.launch.controller_generation < host.controller_generation
    } else {
        transaction.launch.controller_generation == host.controller_generation
    };
    ensure!(
        transaction.transaction_digest != evidence.transaction.transaction_digest
            && transaction.previous_runtime_epoch == evidence.runtime.runtime_epoch
            && expected_generation
            && launch == transaction.launch
            && checkpoint.status == StartupStatus::HostRetired
            && checkpoint.predecessor_runtime_sha256.is_some()
            && checkpoint.successor == Some(successor)
            && host
                .load_durable_manifest(evidence.terminal_id)?
                .recovery_state
                == RecoveryState::Uncertain,
        FIXTURE_RECOVERY_MISMATCH
    );
    let error = host
        .resume_session_locked(evidence.terminal_id, ResumeSelection::ExplicitRecreate)
        .await
        .err()
        .context(FIXTURE_RETRY_MISMATCH)?
        .into_explicit_error();
    ensure!(
        error.code == crate::protocol::error_code::WORKER_START_FAILED
            && error.message == WORKER_START_PUBLIC_DETAIL
            && read_json::<ResumeTransaction>(&host.paths.resume_transaction(evidence.terminal_id))?
                == transaction
            && read_json::<WorkerLaunchSpec>(&host.paths.worker_launch(evidence.terminal_id))?
                == launch
            && read_json::<StartupCheckpointEvidence>(
                &host.paths.worker_startup_checkpoint(evidence.terminal_id)
            )? == checkpoint,
        FIXTURE_RETRY_MISMATCH
    );
    Ok(())
}
