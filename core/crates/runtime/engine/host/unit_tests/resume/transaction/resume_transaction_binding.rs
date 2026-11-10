use crate::host::tests::*;
use super::*;

#[tokio::test]
async fn recreate_binding_clear_replay_preserves_new_runtime_binding() {
    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);
    let manifest = fixture_manifest(directory.path());
    let runtime = persist_dead_resume_session(&host, &manifest);
    persist_archive_exit(&host, &runtime, runtime.runtime_epoch);
    let binding = host_binding(manifest.terminal_id);
    let store = AgentBindingStore::new(host.paths.agent_binding(manifest.terminal_id));
    store.save(&binding).unwrap();

    let error = host
        .resume_session(exact_recreate_request(
            &manifest,
            &runtime,
            TerminalRuntimeStatus::Exited,
        ))
        .await
        .unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::WORKER_START_FAILED);
    assert!(!store.path().exists());
    let transaction: ResumeTransaction =
        read_json(&host.paths.resume_transaction(manifest.terminal_id)).unwrap();
    let (_, previous, _) = binding_to_clear(&transaction).unwrap();
    assert!(previous.is_some());

    let marker: ResumeBindingClearMarker =
        read_json(&host.paths.resume_binding_clear(manifest.terminal_id)).unwrap();
    assert_eq!(marker.phase, ResumeBindingClearPhase::Cleared);
    assert_eq!(&marker.previous, previous);
    let mut replacement = binding;
    replacement.updated_at_ms = replacement.updated_at_ms.saturating_add(1);
    store.save(&replacement).unwrap();

    host.stage_resume_binding_disposition(&transaction).unwrap();
    assert!(host.paths.resume_transaction(manifest.terminal_id).exists());
    host.apply_resume_binding_disposition(&transaction).unwrap();
    assert_eq!(
        read_json::<AgentBinding>(store.path()).unwrap(),
        replacement
    );
    host.complete_resume_binding_disposition(&transaction)
        .unwrap();
    assert!(
        !host
            .paths
            .resume_binding_clear(manifest.terminal_id)
            .exists()
    );
}

#[tokio::test]
async fn durable_resume_recreate_never_clears_a_replaced_binding() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    let binding = host_binding(manifest.terminal_id);
    let store = AgentBindingStore::new(host.paths.agent_binding(manifest.terminal_id));
    store.save(&binding).unwrap();
    let selected = host
        .binding_for_resume(&mut manifest, ResumeSelection::ExplicitRecreate)
        .await
        .unwrap();
    assert!(store.path().exists());
    let launch = shell_resume_launch(&host, &manifest);
    let transaction = ResumeTransaction::new(
        Uuid::new_v4(),
        ResumeIntent::Explicit,
        ResumeTarget::from_binding(&selected),
        &launch,
    )
    .unwrap();
    let mut replacement = binding;
    replacement.updated_at_ms = replacement.updated_at_ms.saturating_add(1);
    store.save(&replacement).unwrap();

    assert!(host.stage_resume_binding_disposition(&transaction).is_err());
    assert_eq!(
        read_json::<AgentBinding>(store.path()).unwrap(),
        replacement
    );
}
