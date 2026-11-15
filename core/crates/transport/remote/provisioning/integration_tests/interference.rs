use super::{constants::*, fixture::*};
use std::{
    future::Future,
    sync::Arc,
    task::{Context, Poll, Waker},
};
use yolo_remote_provisioning_protocol::{ProvisionOutcome, ProvisionRequest};

#[tokio::test]
async fn caller_cancellation_and_concurrent_requests_cannot_acquire_another_submit_permit() {
    let fixture = Fixture::new();
    let service = fixture.service();
    let entered = Arc::new(tokio::sync::Semaphore::new(0));
    let release = Arc::new(tokio::sync::Semaphore::new(0));
    fixture.remote.state.lock().unwrap().gate = Some((entered.clone(), release));
    let operation = service.provision(&fixture.remote, &authorized, &fixture.request, deadline());
    let mut operation = Box::pin(operation);
    let progress = operation
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()));
    assert!(
        matches!(progress, Poll::Pending),
        "provisioning finished before gated enrollment: {progress:?}"
    );
    entered.acquire().await.unwrap().forget();
    assert!(
        service
            .provision(&fixture.remote, &authorized, &fixture.request, deadline())
            .await
            .is_err()
    );
    drop(operation);
    assert!(matches!(
        service
            .recover(&fixture.remote, &authorized, &fixture.request, deadline())
            .await
            .unwrap()
            .fields()
            .outcome,
        ProvisionOutcome::Ready { .. }
    ));
    assert_eq!(fixture.remote.state.lock().unwrap().calls, ONE);
}

#[tokio::test]
async fn identity_reuse_source_substitution_and_journal_replacement_fail_closed() {
    let fixture = Fixture::new();
    let service = fixture.service();
    service
        .provision(&fixture.remote, &authorized, &fixture.request, deadline())
        .await
        .unwrap();
    let mut wrong = fixture.request.fields().clone();
    wrong.host_home = STATE.into();
    wrong.state = HOME.into();
    let wrong = ProvisionRequest::new(wrong).unwrap();
    assert!(
        service
            .recover(&fixture.remote, &authorized, &wrong, deadline())
            .await
            .is_err()
    );
    assert!(
        service
            .provision(&fixture.remote, &authorized, &wrong, deadline())
            .await
            .is_err()
    );
    let file = fixture.file();
    let entry = file.parent().unwrap();
    let saved = fixture.root.path().join(SAVED);
    std::fs::rename(entry, &saved).unwrap();
    yolo_paths::directory::PrivateDirectory::ensure(entry).unwrap();
    std::fs::copy(saved.join(RECORD_FILE), &file).unwrap();
    assert!(
        service
            .recover(&fixture.remote, &authorized, &fixture.request, deadline())
            .await
            .is_err()
    );
    assert_eq!(fixture.remote.bundle.commits(), ONE);
    assert_eq!(fixture.remote.state.lock().unwrap().calls, ONE);
}

#[tokio::test]
async fn missing_operation_is_readonly_and_revoked_admission_never_mutates_remote() {
    let fixture = Fixture::new();
    let service = fixture.service();
    assert!(matches!(
        service
            .recover(&fixture.remote, &authorized, &fixture.request, deadline())
            .await
            .unwrap()
            .fields()
            .outcome,
        ProvisionOutcome::Absent {}
    ));
    assert!(!fixture.file().exists());
    let rejected = || anyhow::bail!(REVOKED);
    assert!(
        service
            .provision(&fixture.remote, &rejected, &fixture.request, deadline())
            .await
            .is_err()
    );
    assert_eq!(fixture.remote.bundle.commits(), 0);
    assert_eq!(fixture.remote.state.lock().unwrap().calls, 0);
}
