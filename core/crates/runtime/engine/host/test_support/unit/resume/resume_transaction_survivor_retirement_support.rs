const SURVIVOR_REARM_FAILED_RECONCILE_COUNT: usize = 1;

fn recreate_survivor_transaction(
    host: &Host,
    manifest: &TerminalManifest,
    previous_runtime_epoch: Uuid,
) -> ResumeTransaction {
    let launch = shell_resume_launch(host, manifest);
    ResumeTransaction::new(
        previous_runtime_epoch,
        ResumeIntent::Explicit,
        ResumeTarget::Shell {
            binding: ResumeBindingDisposition::Clear {
                operation_id: Uuid::new_v4(),
                previous: None,
                codex_authority: None,
            },
        },
        &launch,
    )
    .unwrap()
}

async fn park_recreate_survivor(
    host: &Host,
    manifest: &TerminalManifest,
    transaction: &ResumeTransaction,
    runtime: &RuntimeRecord,
) -> TerminalView {
    persist_survivor_resume(host, manifest, transaction, runtime);
    host.stage_resume_binding_disposition(transaction).unwrap();
    host.resume_session_locked(manifest.terminal_id, ResumeSelection::ExplicitRecreate)
        .await
        .unwrap()
}

fn read_resume_tombstone(host: &Host, terminal_id: Uuid) -> crate::model::TerminationTombstone {
    read_json(&host.paths.terminate_tombstone(terminal_id)).unwrap()
}
