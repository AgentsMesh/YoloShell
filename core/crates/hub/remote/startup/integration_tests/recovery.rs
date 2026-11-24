use super::{recovery_constants::*, recovery_fixture::Fixture};
use crate::{ensure_running, inspect, recover};
use yolo_remote_provisioning_protocol::EnrollmentOutcome;

#[tokio::test]
async fn prepared_before_configure_is_unknown_and_reentry_never_submits_or_starts() {
    let mut fixture = Fixture::new(false).await;
    drop(fixture.prepare());
    let before = std::fs::read(fixture.file()).unwrap();
    let result = recover(&fixture.directories, &fixture.request, deadline())
        .await
        .unwrap();
    assert!(matches!(
        result.fields().outcome,
        EnrollmentOutcome::Unknown {}
    ));
    assert!(
        ensure_running(&fixture.directories, &fixture.request, deadline())
            .await
            .is_err()
    );
    assert_eq!(fixture.configured(), 0);
    assert_eq!(std::fs::read(fixture.file()).unwrap(), before);
    assert!(!fixture.directories.install_root.exists());
    fixture.peer.finish().await;
}

#[tokio::test]
async fn lost_configure_reply_stays_unknown_until_explicit_readonly_recovery() {
    let mut fixture = Fixture::new(true).await;
    let result = crate::enrollment::enroll(
        &fixture.directories,
        &fixture.connection(),
        &fixture.request,
        deadline(),
    )
    .await;
    assert!(result.is_err());
    assert_eq!(fixture.configured(), EXPECTED_CONFIGURE_COUNT);
    let record = inspect(&fixture.directories, &fixture.request)
        .unwrap()
        .unwrap();
    assert!(record.completion.is_none());
    assert_eq!(
        fixture.shared.lock().unwrap().connections,
        EXPECTED_CONNECTION_COUNT
    );
    let before = std::fs::read(fixture.file()).unwrap();
    assert!(matches!(
        recover(&fixture.directories, &fixture.request, deadline())
            .await
            .unwrap()
            .fields()
            .outcome,
        EnrollmentOutcome::Ready { .. }
    ));
    ensure_running(&fixture.directories, &fixture.request, deadline())
        .await
        .unwrap();
    assert_eq!(fixture.configured(), EXPECTED_CONFIGURE_COUNT);
    assert_eq!(std::fs::read(fixture.file()).unwrap(), before);
    fixture.peer.finish().await;
}

#[tokio::test]
async fn applied_configuration_without_completion_receipt_is_recoverable_without_writing() {
    let mut fixture = Fixture::new(false).await;
    let permit = fixture.prepare();
    fixture.apply(permit.prepared());
    drop(permit); // Caller disappeared after Configure, before completion publication.
    let before = std::fs::read(fixture.file()).unwrap();
    let result = recover(&fixture.directories, &fixture.request, deadline())
        .await
        .unwrap();
    let EnrollmentOutcome::Ready { receipt } = &result.fields().outcome else {
        panic!()
    };
    receipt.validate_for(&fixture.request).unwrap();
    assert_eq!(fixture.configured(), 0);
    assert_eq!(std::fs::read(fixture.file()).unwrap(), before);
    assert!(
        inspect(&fixture.directories, &fixture.request)
            .unwrap()
            .unwrap()
            .completion
            .is_none()
    );
    fixture.peer.finish().await;
}

#[tokio::test]
async fn no_change_preparation_is_recorded_and_read_back_without_configure() {
    let mut fixture = Fixture::new(false).await;
    fixture.apply(&fixture.prepared());
    crate::enrollment::enroll(
        &fixture.directories,
        &fixture.connection(),
        &fixture.request,
        deadline(),
    )
    .await
    .unwrap();
    let record = inspect(&fixture.directories, &fixture.request)
        .unwrap()
        .unwrap();
    assert!(record.prepared.change.is_none() && record.completion.is_some());
    assert_eq!(fixture.configured(), 0);
    assert!(matches!(
        recover(&fixture.directories, &fixture.request, deadline())
            .await
            .unwrap()
            .fields()
            .outcome,
        EnrollmentOutcome::Ready { .. }
    ));
    fixture.peer.finish().await;
}
