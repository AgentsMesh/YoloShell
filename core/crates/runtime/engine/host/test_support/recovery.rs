const LIFECYCLE_CONVERGENCE_CORRUPT_RUNTIME: &[u8] = b"invalid-runtime-record";
const LIFECYCLE_CONVERGENCE_DEAD_PID: u32 = u32::MAX;
const LIFECYCLE_CONVERGENCE_DEAD_START: u64 = 1;
const LIFECYCLE_CONVERGENCE_SOCKET_EXTENSION: &str = "sock";

fn lifecycle_convergence_socket(root: &Path, terminal_id: Uuid) -> PathBuf {
    root.join(terminal_id.to_string())
        .with_extension(LIFECYCLE_CONVERGENCE_SOCKET_EXTENSION)
}

fn lifecycle_convergence_dead_runtime(record: &RuntimeRecord) -> RuntimeRecord {
    let mut dead = record.clone();
    dead.worker_pid = LIFECYCLE_CONVERGENCE_DEAD_PID;
    dead.worker_start_time = LIFECYCLE_CONVERGENCE_DEAD_START;
    dead.child_pid = LIFECYCLE_CONVERGENCE_DEAD_PID;
    dead.child_start_time = LIFECYCLE_CONVERGENCE_DEAD_START;
    dead
}

async fn seed_recovery_manifest(host: &Host, manifest: &TerminalManifest) {
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    host.write_namespace_json(&host.paths.manifest(manifest.terminal_id), manifest)
        .unwrap();
    host.catalog_insert(manifest).await.unwrap();
}

fn persist_survivor_resume(
    host: &Host,
    manifest: &TerminalManifest,
    transaction: &ResumeTransaction,
    runtime: &RuntimeRecord,
) {
    persist_host_session(host, manifest, runtime);
    host.publish_resume_transaction(transaction).unwrap();
    host.publish_worker_launch(&transaction.launch).unwrap();
    host.write_namespace_json(
        &host.paths.terminate_tombstone(manifest.terminal_id),
        &crate::model::TerminationTombstone::user_request(
            manifest.terminal_id,
            transaction.previous_runtime_epoch,
            false,
        ),
    )
    .unwrap();
}

fn dead_survivor_rearm_fixture(
    host: &Host,
    directory: &Path,
) -> (TerminalManifest, ResumeTransaction, RuntimeRecord) {
    let mut manifest = fixture_manifest(directory);
    manifest.recovery_state = RecoveryState::Restoring;
    let transaction = recreate_survivor_transaction(host, &manifest, Uuid::new_v4());
    let mut runtime = planned_resume_runtime(
        host,
        &transaction,
        RESUME_TEST_DEAD_WORKER_PID,
        RESUME_TEST_PROCESS_START_TIME,
        RESUME_TEST_DEAD_CHILD_PID,
        RESUME_TEST_PROCESS_START_TIME,
        BootstrapDelivery::Survivor,
    );
    runtime.boot_id = RESUME_TEST_PREVIOUS_BOOT_ID.into();
    persist_survivor_resume(host, &manifest, &transaction, &runtime);
    (manifest, transaction, runtime)
}
