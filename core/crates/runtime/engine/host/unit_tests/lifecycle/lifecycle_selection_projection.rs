use crate::host::tests::*;
use super::*;

#[path = "../../test_support/unit/lifecycle_selection_projection.rs"]
mod selection_projection_fixture;

#[tokio::test]
async fn lifecycle_projection_translates_only_the_verified_public_snapshot() {
    let (_directory, host, manifest, runtime) = selection_projection_fixture::setup(true).await;
    let selected = selection_projection_fixture::selected(&host, &manifest).await;
    assert!(manifest_digest_changed(&selected, &manifest));
    let translated = host
        .durable_product_selection(selected.clone())
        .await
        .unwrap();
    assert!(!manifest_digest_changed(&translated, &manifest));
    assert_eq!(
        translated.expected_runtime_epoch,
        Some(runtime.runtime_epoch)
    );
    assert_eq!(translated.expected_status, selected.expected_status);
    assert_eq!(
        translated.expected_updated_at_ms,
        selected.expected_updated_at_ms
    );
    host.validate_product_selection(&translated).await.unwrap();
    host.revalidate_lifecycle_fence(&translated).await.unwrap();
    let stored = host.catalog_get(manifest.terminal_id).await.unwrap().unwrap();
    assert_manifest_snapshot(&stored, &manifest_snapshot(&manifest));
}

#[tokio::test]
async fn lifecycle_projection_rejects_stale_status_epoch_clock_and_manifest_fields() {
    let (_directory, host, manifest, _) = selection_projection_fixture::setup(true).await;
    let original = selection_projection_fixture::selected(&host, &manifest).await;
    for changed in [
        TerminalLifecycleSelection {
            expected_status: Some(TerminalRuntimeStatus::Running),
            ..original.clone()
        },
        TerminalLifecycleSelection {
            expected_updated_at_ms: Some(manifest.updated_at_ms + 1),
            ..original.clone()
        },
        TerminalLifecycleSelection {
            expected_runtime_epoch: Some(Uuid::new_v4()),
            ..original.clone()
        },
    ] {
        assert_eq!(
            host.durable_product_selection(changed)
                .await
                .unwrap_err()
                .code,
            crate::protocol::error_code::CONFLICT
        );
    }
    let mut changed = manifest.clone();
    changed.recovery_state = RecoveryState::Uncertain;
    changed.archived = true;
    let forged = original.with_expected_manifest_sha256(Some(
        crate::model::TerminalManifestDigest::capture(&changed).unwrap(),
    ));
    assert_eq!(
        host.durable_product_selection(forged)
            .await
            .unwrap_err()
            .code,
        crate::protocol::error_code::CONFLICT
    );
}

#[tokio::test]
async fn lifecycle_projection_does_not_authorize_a_fabricated_dead_worker_view() {
    let (_directory, host, manifest, runtime) = selection_projection_fixture::setup(false).await;
    let mut projected = manifest.clone();
    projected.recovery_state = RecoveryState::Uncertain;
    let selection = selection_projection_fixture::selection(&projected, &runtime);
    assert_eq!(
        host.durable_product_selection(selection)
            .await
            .unwrap_err()
            .message,
        LIFECYCLE_SELECTION_CHANGED
    );
}

#[tokio::test]
async fn lifecycle_projection_preserves_existing_unconditional_and_durable_fences() {
    let (_directory, host, manifest, runtime) = selection_projection_fixture::setup(true).await;
    for selection in [
        TerminalLifecycleSelection::unconditional(manifest.terminal_id),
        selection_projection_fixture::selection(&manifest, &runtime),
    ] {
        assert_eq!(
            host.durable_product_selection(selection.clone())
                .await
                .unwrap(),
            selection
        );
    }
}

#[tokio::test]
async fn lifecycle_projection_keeps_post_translation_mutation_revalidation() {
    let (_directory, host, manifest, _) = selection_projection_fixture::setup(true).await;
    let selected = selection_projection_fixture::selected(&host, &manifest).await;
    let translated = host.durable_product_selection(selected).await.unwrap();
    let mut replacement = manifest.clone();
    replacement.archived = true;
    host.catalog_upsert(&replacement).await.unwrap();
    assert_eq!(
        host.revalidate_lifecycle_fence(&translated)
            .await
            .unwrap_err()
            .code,
        crate::protocol::error_code::CONFLICT
    );
}
