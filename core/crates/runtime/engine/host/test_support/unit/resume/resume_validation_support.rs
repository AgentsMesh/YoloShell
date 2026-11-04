fn provider_resume_fixture(
    host: &Host,
    manifest: &TerminalManifest,
) -> (ResumeBinding, WorkerLaunchSpec, ResumeTransaction) {
    let binding = host_binding(manifest.terminal_id);
    let selected = ResumeBinding::Provider(binding.clone());
    let launch = host
        .prepare_worker_launch(
            manifest,
            selected.launch_command(manifest),
            Some((&binding, ResumeIntent::Explicit)),
            None,
        )
        .unwrap();
    let transaction = ResumeTransaction::new(
        Uuid::new_v4(),
        ResumeIntent::Explicit,
        ResumeTarget::from_binding(&selected),
        &launch,
    )
    .unwrap();
    (selected, launch, transaction)
}

fn retarget_resume_transaction(
    transaction: &ResumeTransaction,
    launch: WorkerLaunchSpec,
) -> ResumeTransaction {
    let mut changed = transaction.clone();
    changed.launch_digest = resume_launch_digest(&launch).unwrap();
    changed.launch = launch;
    changed
}
