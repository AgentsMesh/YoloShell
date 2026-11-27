use super::support::*;
use yolo_hub_client_access_registry::AccessRegistry;

#[tokio::test]
async fn revoked_wakes_existing_waiters_and_cannot_miss_an_earlier_replacement() {
    for replace_first in [false, true] {
        let original = configuration();
        let registry = AccessRegistry::new(original.clone()).unwrap();
        let admission = registry.admit_verified_key(&CLIENT_KEY, NOW_MS).unwrap();
        let mut first = None;
        if !replace_first {
            let waiting = admission.clone();
            first = Some(tokio::spawn(async move { waiting.revoked().await }));
            tokio::task::yield_now().await;
        }
        registry
            .replace(original.revision, next(&original))
            .unwrap();
        tokio::time::timeout(WAKE_TIMEOUT, admission.revoked())
            .await
            .unwrap();
        if let Some(waiting) = first {
            tokio::time::timeout(WAKE_TIMEOUT, waiting)
                .await
                .unwrap()
                .unwrap();
        }
        assert!(!admission.is_current(NOW_MS));
    }
}

#[tokio::test]
async fn explicit_close_revokes_clones_but_rejected_updates_do_not() {
    let original = configuration();
    let registry = AccessRegistry::new(original.clone()).unwrap();
    let first = registry.admit_verified_key(&CLIENT_KEY, NOW_MS).unwrap();
    let second = first.clone();
    let waiting = tokio::spawn(async move { second.revoked().await });
    tokio::task::yield_now().await;
    assert!(registry.replace(original.revision, original).is_err());
    tokio::task::yield_now().await;
    assert!(!waiting.is_finished());
    assert!(first.is_current(NOW_MS));
    registry.clone().close();
    tokio::time::timeout(WAKE_TIMEOUT, waiting)
        .await
        .unwrap()
        .unwrap();
    assert!(!first.is_current(NOW_MS));
}

#[tokio::test]
async fn last_registry_drop_revokes_without_crossing_into_an_identical_new_registry() {
    let original = configuration();
    let registry = AccessRegistry::new(original.clone()).unwrap();
    let retained = registry.clone();
    let previous = registry.admit_verified_key(&CLIENT_KEY, NOW_MS).unwrap();
    drop(registry);
    assert!(previous.is_current(NOW_MS));
    let next_registry = AccessRegistry::new(original).unwrap();
    let current = next_registry
        .admit_verified_key(&CLIENT_KEY, NOW_MS)
        .unwrap();
    drop(retained);
    tokio::time::timeout(WAKE_TIMEOUT, previous.revoked())
        .await
        .unwrap();
    assert!(!previous.is_current(NOW_MS));
    assert!(current.is_current(NOW_MS));
}

#[tokio::test]
async fn expiry_is_immediate_for_authority_checks_and_not_faked_as_a_revision_event() {
    let registry = AccessRegistry::new(configuration()).unwrap();
    let admission = registry.admit_verified_key(&CLIENT_KEY, NOW_MS).unwrap();
    assert!(!admission.is_current(EXPIRES_AT_MS));
    let expired = admission.clone();
    let waiting = tokio::spawn(async move { expired.revoked().await });
    tokio::task::yield_now().await;
    assert!(!waiting.is_finished());
    registry.close();
    tokio::time::timeout(WAKE_TIMEOUT, waiting)
        .await
        .unwrap()
        .unwrap();
}
