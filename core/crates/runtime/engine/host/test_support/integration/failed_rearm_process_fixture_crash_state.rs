struct CrashSnapshot {
    transaction: ResumeTransaction,
}

fn verify_crash_state(
    home: &Path,
    boundary: Boundary,
    evidence: &PreparedEvidence,
) -> Result<CrashSnapshot> {
    evidence.validate_home(home)?;
    let paths = Paths::new(home);
    let transaction: ResumeTransaction = read_json(&paths.resume_transaction(evidence.terminal_id))?;
    let launch = read_optional::<WorkerLaunchSpec>(&paths.worker_launch(evidence.terminal_id))?;
    let manifest: TerminalManifest = read_json(&paths.manifest(evidence.terminal_id))?;
    let runtime: RuntimeRecord = read_json(&paths.runtime_record(evidence.terminal_id))?;
    let tombstone: crate::model::TerminationTombstone =
        read_json(&paths.terminate_tombstone(evidence.terminal_id))?;
    let checkpoint: StartupCheckpointEvidence =
        read_json(&paths.worker_startup_checkpoint(evidence.terminal_id))?;
    let replacement_published = replacement_published(boundary);
    let predecessor_transaction_preserved = transaction == evidence.transaction;
    let expected_launch = match boundary {
        Boundary::TombstoneRolled => Some(&evidence.transaction.launch),
        Boundary::ReplacementLaunchPublished => Some(&transaction.launch),
        _ => None,
    };
    let expected_recovery = match boundary {
        Boundary::TombstoneRolled | Boundary::PreviousLaunchRemoved => RecoveryState::Restoring,
        _ => RecoveryState::Uncertain,
    };
    ensure!(
        predecessor_transaction_preserved == !replacement_published
            && launch.as_ref() == expected_launch
            && manifest.recovery_state == expected_recovery
            && runtime == evidence.runtime
            && tombstone.runtime_epoch == evidence.runtime.runtime_epoch
            && checkpoint == evidence.checkpoint
            && !evidence.socket_path().exists(),
        FIXTURE_CRASH_STATE_MISMATCH
    );
    if replacement_published {
        ensure!(
            transaction.transaction_digest != evidence.transaction.transaction_digest
                && transaction.previous_runtime_epoch == evidence.runtime.runtime_epoch
                && transaction.launch.controller_generation > evidence.controller_generation,
            FIXTURE_CRASH_STATE_MISMATCH
        );
    }
    Ok(CrashSnapshot { transaction })
}

fn replacement_published(boundary: Boundary) -> bool {
    matches!(
        boundary,
        Boundary::ReplacementTransactionPublished | Boundary::ReplacementLaunchPublished
    )
}

fn read_optional<T: serde::de::DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    if path.exists() {
        read_json(path).map(Some)
    } else {
        Ok(None)
    }
}
