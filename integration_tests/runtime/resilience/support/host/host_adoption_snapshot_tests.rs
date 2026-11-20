use super::*;

#[path = "host_adoption_snapshot_support.rs"]
mod support;
use support::*;
#[path = "host_adoption_snapshot_adversarial_tests.rs"]
mod adversarial;

#[tokio::test]
async fn already_published_successor_uses_one_coherent_retained_record() {
    let fixture = SnapshotFixture::new();
    let result = inspect_runtime(&fixture.expected, SUCCESSOR_GENERATION, || async {
        Ok(fixture.view())
    })
    .await
    .unwrap();
    assert_eq!(result, AdoptionProgress::Complete);
}

#[tokio::test]
async fn publication_during_the_view_request_retries_then_completes() {
    let fixture = SnapshotFixture::new();
    let changed = fixture.changed_dimensions();
    let result = inspect_runtime(&fixture.expected, SUCCESSOR_GENERATION, || async {
        fixture.publish(&changed);
        Ok(fixture.view())
    })
    .await
    .unwrap();
    assert!(matches!(result, AdoptionProgress::Pending(ref detail)
        if detail.contains(ADOPTION_PUBLICATION_CHANGED_DETAIL)));
    assert_eq!(fixture.expected.observed.lock().unwrap().record(), &changed);

    assert_eq!(
        inspect_runtime(&fixture.expected, SUCCESSOR_GENERATION, || async {
            Ok(fixture.view())
        })
        .await
        .unwrap(),
        AdoptionProgress::Complete
    );
}

#[tokio::test]
async fn identical_controller_republication_keeps_a_coherent_current_file_binding() {
    let fixture = SnapshotFixture::new();
    assert_eq!(
        inspect_runtime(&fixture.expected, SUCCESSOR_GENERATION, || async {
            fixture.publish(&fixture.record);
            Ok(fixture.view())
        })
        .await
        .unwrap(),
        AdoptionProgress::Complete
    );
    fixture
        .expected
        .observed
        .lock()
        .unwrap()
        .verify_runtime_file()
        .unwrap();
}

#[tokio::test]
async fn bootstrap_publication_between_view_and_record_requires_a_fresh_view() {
    use yolo_runtime::model::BootstrapDelivery;
    let fixture = SnapshotFixture::with_bootstrap(BootstrapDelivery::Pending);
    let mut changed = fixture.record.clone();
    changed.bootstrap_delivery = BootstrapDelivery::Delivered;
    let result = inspect_runtime(&fixture.expected, SUCCESSOR_GENERATION, || async {
        fixture.publish(&changed);
        Ok(fixture.view())
    })
    .await
    .unwrap();
    assert!(matches!(result, AdoptionProgress::Pending(_)));
    let current = crate::support::runtime_record_test_support::session_view(&changed);
    assert_eq!(
        inspect_runtime(&fixture.expected, SUCCESSOR_GENERATION, || async {
            Ok(current)
        })
        .await
        .unwrap(),
        AdoptionProgress::Complete
    );
}

#[tokio::test]
async fn predecessor_snapshot_never_queries_or_claims_successor_adoption() {
    let fixture = SnapshotFixture::new();
    fixture.publish(fixture.expected.record());
    let queried = std::cell::Cell::new(false);
    let result = inspect_runtime(&fixture.expected, SUCCESSOR_GENERATION, || {
        queried.set(true);
        std::future::ready(Ok(fixture.view()))
    })
    .await
    .unwrap();
    assert!(matches!(result, AdoptionProgress::Pending(_)));
    assert!(!queried.get());
    fixture.publish(&fixture.record);
    assert_eq!(
        inspect_runtime(&fixture.expected, SUCCESSOR_GENERATION, || async {
            Ok(fixture.view())
        })
        .await
        .unwrap(),
        AdoptionProgress::Complete
    );
}

#[tokio::test]
async fn failed_query_preserves_the_observed_generation_for_later_rounds() {
    let fixture = SnapshotFixture::new();
    let result = inspect_runtime(&fixture.expected, SUCCESSOR_GENERATION, || async {
        Err(SnapshotFixture::query_error())
    })
    .await
    .unwrap();
    assert!(matches!(result, AdoptionProgress::Pending(_)));
    let next_round = fixture.expected.clone();
    fixture.publish(fixture.expected.record());
    let queried = std::cell::Cell::new(false);
    let error = inspect_runtime(&next_round, SUCCESSOR_GENERATION, || {
        queried.set(true);
        std::future::ready(Ok(fixture.view()))
    })
    .await
    .unwrap_err();
    assert!(error.contains(ADOPTION_RECORD_CHANGED_DETAIL));
    assert!(error.contains("rolled back its controller generation"));
    assert!(!queried.get());
}
