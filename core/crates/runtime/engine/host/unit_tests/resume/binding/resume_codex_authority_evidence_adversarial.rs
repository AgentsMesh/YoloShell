use crate::host::tests::*;
use super::*;

#[tokio::test]
async fn pending_clear_replay_finishes_after_authority_revocation() {
    let (_directory, host, mut manifest, binding, store) = codex_repair_fixture();
    publish_confirmed_exit_authority(&host, &binding, &store);
    let transaction = codex_recreate_transaction(&host, &mut manifest).await;
    let marker = binding_clear_marker(&transaction, ResumeBindingClearPhase::Pending).unwrap();
    host.write_namespace_json(
        &host.paths.resume_binding_clear(manifest.terminal_id),
        &marker,
    )
    .unwrap();
    let (_, _, authority) = binding_to_clear(&transaction).unwrap();
    let guard = crate::paths::durable_mutation::SessionDurableMutationGuard::acquire_participant(
        &host.paths,
        &host.namespace,
        manifest.terminal_id,
    )
    .unwrap();
    host.revoke_resume_codex_authority(&guard, &transaction, &marker, authority.as_ref())
        .unwrap();
    drop(guard);

    assert!(store.path().exists());
    assert_codex_authority_status(
        &host,
        &store,
        crate::agent::codex::authority::CodexAuthorityStatus::Revoked,
    );
    host.stage_resume_binding_disposition(&transaction).unwrap();

    assert!(!store.path().exists());
    let persisted: ResumeBindingClearMarker = host
        .read_namespace_json(&host.paths.resume_binding_clear(manifest.terminal_id))
        .unwrap();
    assert_eq!(persisted.phase, ResumeBindingClearPhase::Cleared);
}

#[tokio::test]
async fn parsed_authority_same_value_replacement_is_not_revoked() {
    let (_directory, host, mut manifest, binding, store) = codex_repair_fixture();
    publish_confirmed_exit_authority(&host, &binding, &store);
    let transaction = codex_recreate_transaction(&host, &mut manifest).await;
    let (authority, captured_identity) = parsed_authority_evidence(&transaction);
    let authority_store = store.codex_authority_store().unwrap();
    atomic_write_json(authority_store.path(), &authority).unwrap();
    let (_, replacement_identity) = host
        .namespace
        .read_json_with_identity::<crate::agent::codex::authority::CodexBindingAuthority>(
            authority_store.path(),
        )
        .unwrap();
    assert!(!replacement_identity.unchanged(captured_identity));

    assert!(host.stage_resume_binding_disposition(&transaction).is_err());
    assert!(store.path().exists());
    assert_eq!(
        authority_store.load_in_namespace(&host.namespace).unwrap(),
        Some(authority)
    );
}

#[tokio::test]
async fn raw_authority_same_value_replacement_is_not_removed() {
    let (_directory, host, mut manifest, binding, store) = codex_repair_fixture();
    ProviderAuthorityPublisher::new()
        .publish(&host, &binding)
        .unwrap();
    let authority_store = store.codex_authority_store().unwrap();
    let malformed = serde_json::json!({});
    atomic_write_json(authority_store.path(), &malformed).unwrap();
    let transaction = codex_recreate_transaction(&host, &mut manifest).await;
    let captured_identity = raw_authority_evidence(&transaction);
    atomic_write_json(authority_store.path(), &malformed).unwrap();
    let (_, replacement_identity) = host
        .namespace
        .read_json_with_identity::<serde_json::Value>(authority_store.path())
        .unwrap();
    assert!(!replacement_identity.unchanged(captured_identity));

    assert!(host.stage_resume_binding_disposition(&transaction).is_err());
    assert!(store.path().exists());
    assert_eq!(
        read_json::<serde_json::Value>(authority_store.path()).unwrap(),
        malformed
    );
}

#[tokio::test]
async fn foreign_session_authority_is_isolated_before_recreate_capture() {
    let (_directory, host, mut manifest, binding, store) = codex_repair_fixture();
    publish_confirmed_exit_authority(&host, &binding, &store);
    let authority_store = store.codex_authority_store().unwrap();
    let mut foreign = authority_store
        .load_in_namespace(&host.namespace)
        .unwrap()
        .unwrap();
    foreign.binding.terminal_id = Uuid::new_v4();
    let foreign = foreign
        .with_verified_resume_identity(&host.boot_id)
        .unwrap();
    foreign.validate().unwrap();
    host.write_namespace_json(authority_store.path(), &foreign)
        .unwrap();

    assert!(
        host.binding_for_resume(&mut manifest, ResumeSelection::ExplicitRecreate)
            .await
            .is_err()
    );
    assert_eq!(
        authority_store.load_in_namespace(&host.namespace).unwrap(),
        Some(foreign)
    );
    assert!(store.path().exists());
}

#[tokio::test]
async fn parsed_authority_deletion_is_converged_to_a_durable_revocation() {
    let (_directory, host, mut manifest, binding, store) = codex_repair_fixture();
    publish_confirmed_exit_authority(&host, &binding, &store);
    let transaction = codex_recreate_transaction(&host, &mut manifest).await;
    let authority_store = store.codex_authority_store().unwrap();
    host.namespace.remove_file(authority_store.path()).unwrap();

    host.stage_resume_binding_disposition(&transaction).unwrap();

    assert!(!store.path().exists());
    assert_codex_authority_status(
        &host,
        &store,
        crate::agent::codex::authority::CodexAuthorityStatus::Revoked,
    );
    let marker: ResumeBindingClearMarker = host
        .read_namespace_json(&host.paths.resume_binding_clear(manifest.terminal_id))
        .unwrap();
    assert_eq!(marker.phase, ResumeBindingClearPhase::Cleared);
}

pub(super) async fn codex_recreate_transaction(
    host: &Host,
    manifest: &mut TerminalManifest,
) -> ResumeTransaction {
    let selected = host
        .binding_for_resume(manifest, ResumeSelection::ExplicitRecreate)
        .await
        .unwrap();
    ResumeTransaction::new(
        Uuid::new_v4(),
        ResumeIntent::Explicit,
        ResumeTarget::from_binding(&selected),
        &shell_resume_launch(host, manifest),
    )
    .unwrap()
}

fn parsed_authority_evidence(
    transaction: &ResumeTransaction,
) -> (
    crate::agent::codex::authority::CodexBindingAuthority,
    crate::paths::identity::FileIdentity,
) {
    let (_, _, evidence) = binding_to_clear(transaction).unwrap();
    let Some(ResumeCodexAuthorityEvidence::Parsed {
        authority,
        file_identity,
    }) = evidence
    else {
        unreachable!();
    };
    (authority.clone(), *file_identity)
}

pub(super) fn raw_authority_evidence(transaction: &ResumeTransaction) -> crate::paths::identity::FileIdentity {
    let (_, _, evidence) = binding_to_clear(transaction).unwrap();
    let Some(ResumeCodexAuthorityEvidence::Raw { file_identity }) = evidence else {
        unreachable!();
    };
    *file_identity
}

fn assert_codex_authority_status(
    host: &Host,
    store: &AgentBindingStore,
    expected: crate::agent::codex::authority::CodexAuthorityStatus,
) {
    let authority = store
        .codex_authority_store()
        .unwrap()
        .load_in_namespace(&host.namespace)
        .unwrap()
        .unwrap();
    assert_eq!(authority.status, expected);
}
