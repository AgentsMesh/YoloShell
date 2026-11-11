use crate::host::tests::*;
use super::*;

#[derive(Clone, Copy)]
enum RawAuthorityReplayDamage {
    Transaction,
    Marker,
    Launch,
}

#[tokio::test]
async fn present_raw_authority_requires_every_exact_replay_artifact() {
    for damage in replay_boundary_damage_cases() {
        let (_directory, host, manifest, store, transaction, marker) =
            raw_authority_replay_fixture().await;
        let authority_store = store.codex_authority_store().unwrap();
        let captured = raw_authority_evidence(&transaction);
        replace_raw_authority_replay_artifact(&host, &transaction, &marker, damage);
        let guard = crate::paths::durable_mutation::SessionDurableMutationGuard::acquire_participant(
            &host.paths,
            &host.namespace,
            manifest.terminal_id,
        )
        .unwrap();

        let result = host.remove_raw_codex_authority(
            &guard,
            &authority_store,
            captured,
            &transaction,
            &marker,
        );

        assert!(result.is_err());
        assert_raw_authority_unchanged(&host, &authority_store, captured);
        assert!(store.path().exists());
    }
}

#[tokio::test]
async fn present_raw_authority_revalidates_every_artifact_at_commit() {
    for damage in replay_boundary_damage_cases() {
        let (_directory, host, manifest, store, transaction, marker) =
            raw_authority_replay_fixture().await;
        let authority_store = store.codex_authority_store().unwrap();
        let captured = raw_authority_evidence(&transaction);
        let guard = crate::paths::durable_mutation::SessionDurableMutationGuard::acquire_participant(
            &host.paths,
            &host.namespace,
            manifest.terminal_id,
        )
        .unwrap();
        let injected = std::cell::Cell::new(false);

        let result = host.remove_raw_codex_authority_with_boundary(
            &guard,
            &authority_store,
            captured,
            &transaction,
            &marker,
            || {
                if !injected.replace(true) {
                    replace_raw_authority_replay_artifact(
                        &host,
                        &transaction,
                        &marker,
                        damage,
                    );
                }
                Ok(())
            },
        );

        assert!(result.is_err());
        assert_raw_authority_unchanged(&host, &authority_store, captured);
        assert!(store.path().exists());
    }
}

#[tokio::test]
async fn missing_raw_authority_requires_every_exact_replay_artifact() {
    for damage in replay_boundary_damage_cases() {
        let (_directory, host, manifest, store, transaction, marker) =
            raw_authority_replay_fixture().await;
        let authority_store = store.codex_authority_store().unwrap();
        let captured = raw_authority_evidence(&transaction);
        host.namespace.remove_file(authority_store.path()).unwrap();
        replace_raw_authority_replay_artifact(&host, &transaction, &marker, damage);
        let guard = crate::paths::durable_mutation::SessionDurableMutationGuard::acquire_participant(
            &host.paths,
            &host.namespace,
            manifest.terminal_id,
        )
        .unwrap();

        let result = host.remove_raw_codex_authority(
            &guard,
            &authority_store,
            captured,
            &transaction,
            &marker,
        );

        assert!(result.is_err());
        assert!(
            host.namespace
                .file_is_missing(authority_store.path())
                .unwrap()
        );
        assert!(store.path().exists());
    }
}

#[tokio::test]
async fn same_value_replay_artifact_aba_fails_snapshot_revalidation() {
    let (_directory, host, manifest, store, transaction, _marker) =
        raw_authority_replay_fixture().await;
    let guard = crate::paths::durable_mutation::SessionDurableMutationGuard::acquire_participant(
        &host.paths,
        &host.namespace,
        manifest.terminal_id,
    )
    .unwrap();
    let path = host.paths.resume_transaction(manifest.terminal_id);
    let (_, identity) = read_exact_raw_codex_replay_artifact(&guard, &path, &transaction).unwrap();

    host.write_namespace_json(&path, &transaction).unwrap();

    assert!(
        require_raw_codex_replay_artifact_unchanged(&guard, &path, &transaction, identity).is_err()
    );
    assert!(store.path().exists());
}

pub(super) const fn replay_boundary_damage_cases() -> [RawAuthorityReplayDamage; 3] {
    [
        RawAuthorityReplayDamage::Transaction,
        RawAuthorityReplayDamage::Marker,
        RawAuthorityReplayDamage::Launch,
    ]
}

fn assert_raw_authority_unchanged(
    host: &Host,
    store: &CodexBindingAuthorityStore,
    expected: FileIdentity,
) {
    let current = host
        .namespace
        .file(store.path())
        .unwrap()
        .inspect(RESUME_BINDING_EVIDENCE_MAX_BYTES)
        .unwrap()
        .unwrap();
    assert!(current.unchanged(expected));
}

fn replace_raw_authority_replay_artifact(
    host: &Host,
    transaction: &ResumeTransaction,
    marker: &ResumeBindingClearMarker,
    damage: RawAuthorityReplayDamage,
) {
    match damage {
        RawAuthorityReplayDamage::Transaction => {
            let replacement = ResumeTransaction::new(
                Uuid::new_v4(),
                transaction.intent,
                transaction.target.clone(),
                &transaction.launch,
            )
            .unwrap();
            host.write_namespace_json(
                &host.paths.resume_transaction(transaction.terminal_id),
                &replacement,
            )
            .unwrap();
        }
        RawAuthorityReplayDamage::Marker => {
            let mut replacement = marker.clone();
            replacement.operation_id = Uuid::new_v4();
            host.write_namespace_json(
                &host.paths.resume_binding_clear(transaction.terminal_id),
                &replacement,
            )
            .unwrap();
        }
        RawAuthorityReplayDamage::Launch => {
            let mut replacement = transaction.launch.clone();
            replacement.runtime_epoch = Uuid::new_v4();
            host.write_namespace_json(
                &host.paths.worker_launch(transaction.terminal_id),
                &replacement,
            )
            .unwrap();
        }
    }
}
