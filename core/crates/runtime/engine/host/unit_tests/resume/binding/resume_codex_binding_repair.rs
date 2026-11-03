use crate::host::tests::*;
use super::*;

#[tokio::test]
async fn preserve_resume_repairs_an_active_codex_binding_publication_gap() {
    for intent in [ResumeIntent::Automatic, ResumeIntent::Explicit] {
        let (directory, host, mut manifest, mut binding, store) = codex_repair_fixture();
        ProviderAuthorityPublisher::new()
            .publish(&host, &binding)
            .unwrap();
        host.namespace.remove_file(store.path()).unwrap();

        let selected = host
            .binding_for_resume(&mut manifest, ResumeSelection::Preserve(intent))
            .await
            .unwrap();
        binding.state = crate::agent::AgentState::Unknown;
        assert!(matches!(selected, ResumeBinding::Provider(ref value) if value == &binding));
        assert_eq!(
            store.read_raw_in_namespace(&host.namespace).unwrap(),
            Some(binding)
        );
        drop(directory);
    }
}

#[tokio::test]
async fn preserve_resume_repairs_only_the_safe_stale_codex_predecessor() {
    let (_directory, host, mut manifest, mut target, store) = codex_repair_fixture();
    let stale = target.clone();
    target.state = crate::agent::AgentState::WaitingForInput;
    target.updated_at_ms = target
        .updated_at_ms
        .saturating_add(CODEX_REPAIR_TIMESTAMP_INCREMENT);
    ProviderAuthorityPublisher::new()
        .publish(&host, &target)
        .unwrap();
    host.write_namespace_json(store.path(), &stale).unwrap();

    let selected = host
        .binding_for_resume(
            &mut manifest,
            ResumeSelection::Preserve(ResumeIntent::Automatic),
        )
        .await
        .unwrap();
    target.state = crate::agent::AgentState::Unknown;
    assert!(matches!(selected, ResumeBinding::Provider(ref value) if value == &target));
    assert_eq!(
        store.read_raw_in_namespace(&host.namespace).unwrap(),
        Some(target)
    );
}

#[tokio::test]
async fn preserve_resume_repairs_the_canonical_unavailable_predecessor() {
    for intent in [ResumeIntent::Automatic, ResumeIntent::Explicit] {
        let (_directory, host, mut manifest, mut target, store) = codex_repair_fixture();
        ProviderAuthorityPublisher::new()
            .publish(&host, &target)
            .unwrap();
        let mut unavailable = target.clone();
        unavailable.state = crate::agent::AgentState::Unknown;
        unavailable.recovery_availability = crate::agent::AgentRecoveryAvailability::Unavailable;
        host.write_namespace_json(store.path(), &unavailable)
            .unwrap();

        let selected = host
            .binding_for_resume(&mut manifest, ResumeSelection::Preserve(intent))
            .await
            .unwrap();
        target.state = crate::agent::AgentState::Unknown;
        assert!(matches!(selected, ResumeBinding::Provider(ref value) if value == &target));
        assert_eq!(
            store.read_raw_in_namespace(&host.namespace).unwrap(),
            Some(target)
        );
    }
}

#[tokio::test]
async fn explicit_recreate_revocation_survives_marker_retirement() {
    let (_directory, host, mut manifest, binding, store) = codex_repair_fixture();
    publish_confirmed_exit_authority(&host, &binding, &store);
    let selected = host
        .binding_for_resume(&mut manifest, ResumeSelection::ExplicitRecreate)
        .await
        .unwrap();
    let transaction = ResumeTransaction::new(
        Uuid::new_v4(),
        ResumeIntent::Explicit,
        ResumeTarget::from_binding(&selected),
        &shell_resume_launch(&host, &manifest),
    )
    .unwrap();

    host.stage_resume_binding_disposition(&transaction).unwrap();
    host.apply_resume_binding_disposition(&transaction).unwrap();
    host.complete_resume_binding_disposition(&transaction)
        .unwrap();

    assert!(!store.path().exists());
    let authority = store
        .codex_authority_store()
        .unwrap()
        .load_in_namespace(&host.namespace)
        .unwrap()
        .unwrap();
    assert_eq!(
        authority.status,
        crate::agent::codex::authority::CodexAuthorityStatus::Revoked
    );
    assert!(
        !host
            .paths
            .resume_binding_clear(manifest.terminal_id)
            .exists()
    );
    let resumed = host
        .binding_for_resume(
            &mut manifest,
            ResumeSelection::Preserve(ResumeIntent::Explicit),
        )
        .await;
    assert!(resumed.is_err());
    assert!(!store.path().exists());
}

pub(super) fn codex_repair_fixture() -> (
    tempfile::TempDir,
    Host,
    TerminalManifest,
    AgentBinding,
    AgentBindingStore,
) {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    persist_dead_resume_session(&host, &manifest);
    let binding = completion_provider_binding(&directory, manifest.terminal_id);
    let store = AgentBindingStore::new(host.paths.agent_binding(manifest.terminal_id));
    (directory, host, manifest, binding, store)
}

pub(super) fn publish_confirmed_exit_authority(
    host: &Host,
    running: &AgentBinding,
    store: &AgentBindingStore,
) {
    let mut exited = running.clone();
    exited.state = crate::agent::AgentState::Exited;
    exited.process_pid = None;
    exited.process_start_time = None;
    let runtime = host
        .agent_authority_runtime(running.terminal_id)
        .unwrap()
        .unwrap();
    let provider = crate::agent::hooks::ProcessIdentity::new(
        running.process_pid.unwrap(),
        running.process_start_time.unwrap(),
    );
    let authority =
        crate::agent::codex::authority::CodexBindingAuthority::new_with_verified_resume_identity(
            runtime,
            provider,
            crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
            crate::agent::codex::authority::CodexAuthorityVersion {
                route_revision: PROVIDER_AUTHORITY_FIRST_ROUTE_REVISION,
                invocation_id: Uuid::new_v4(),
                generation: crate::agent::hooks::Generation::INITIAL,
                authority_revision: PROVIDER_AUTHORITY_REVISION,
                sequence: PROVIDER_AUTHORITY_SEQUENCE,
            },
            crate::agent::codex::authority::CodexAuthorityStatus::Active,
            exited,
            &host.boot_id,
        )
        .unwrap();
    store
        .codex_authority_store()
        .unwrap()
        .publish_in_namespace_with_commit_fence(
            &host.namespace,
            store.path(),
            &authority,
            || Ok(()),
        )
        .unwrap();
}
const CODEX_REPAIR_TIMESTAMP_INCREMENT: i64 = 1;
