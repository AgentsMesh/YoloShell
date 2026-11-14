use crate::host::tests::*;
const HOST_RESPONSE_DRAIN_TEST_DETAIL: &str = "response drain test fail-stop";
const HOST_RESPONSE_DRAIN_TEST_TIMEOUT: Duration = Duration::from_secs(1);

#[test]
fn host_response_drain_deadline_covers_every_admitted_request_contract() {
    assert_eq!(
        HOST_RESPONSE_DRAIN_TIMEOUT,
        crate::session_timing::SESSION_RESUME_CLIENT_TIMEOUT
    );
    assert!(
        crate::session_timing::SESSION_STARTUP_CLIENT_TIMEOUT < HOST_RESPONSE_DRAIN_TIMEOUT
    );
    assert!(
        crate::session_timing::SESSION_LIFECYCLE_CLIENT_TIMEOUT <= HOST_RESPONSE_DRAIN_TIMEOUT
    );
}

#[tokio::test]
async fn host_response_drain_waits_for_admitted_response_and_rejects_late_admission() {
    let drain = request_drain::HostRequestDrain::default();
    let ticket = drain.admit_response().unwrap();

    drain.fail_stop(HOST_RESPONSE_DRAIN_TEST_DETAIL);
    assert!(drain.admit_response().is_err());
    assert!(!drain.wait_for_response_drain(Duration::ZERO).await);

    ticket.publish();
    assert!(drain
        .wait_for_response_drain(HOST_RESPONSE_DRAIN_TEST_TIMEOUT)
        .await);
}

#[tokio::test]
async fn host_response_drain_cancelled_publication_releases_fail_stop_barrier() {
    let drain = request_drain::HostRequestDrain::default();
    let ticket = drain.admit_response().unwrap();
    drain.fail_stop(HOST_RESPONSE_DRAIN_TEST_DETAIL);

    drop(ticket);

    assert!(drain
        .wait_for_response_drain(HOST_RESPONSE_DRAIN_TEST_TIMEOUT)
        .await);
}

#[tokio::test(start_paused = true)]
async fn host_response_drain_permanently_blocked_publication_is_bounded() {
    let drain = request_drain::HostRequestDrain::default();
    let _ticket = drain.admit_response().unwrap();
    drain.fail_stop(HOST_RESPONSE_DRAIN_TEST_DETAIL);

    assert!(!await_host_response_drain(&drain, HOST_RESPONSE_DRAIN_TEST_TIMEOUT).await);
}

#[tokio::test]
async fn host_response_drain_concurrent_fail_stops_commit_one_admission_boundary() {
    let drain = request_drain::HostRequestDrain::default();
    let first = drain.clone();
    let second = drain.clone();

    let (first, second) = tokio::join!(
        tokio::spawn(async move { first.fail_stop(HOST_RESPONSE_DRAIN_TEST_DETAIL) }),
        tokio::spawn(async move { second.fail_stop(HOST_RESPONSE_DRAIN_TEST_DETAIL) }),
    );

    first.unwrap();
    second.unwrap();
    assert!(drain.admit_response().is_err());
    assert!(drain
        .wait_for_response_drain(HOST_RESPONSE_DRAIN_TEST_TIMEOUT)
        .await);
}

#[tokio::test]
async fn host_response_drain_ingress_waits_before_aborting_client_task() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let drain = host.request_drain.clone();
    let ticket = drain.admit_response().unwrap();
    let mut tasks = tokio::task::JoinSet::new();
    let (client_started, started) = tokio::sync::oneshot::channel();
    tasks.spawn(async move {
        let _ = client_started.send(());
        std::future::pending::<()>().await;
    });
    started.await.unwrap();
    let draining_host = Arc::clone(&host);
    let draining = tokio::spawn(async move {
        drain_host_clients(&draining_host, &mut tasks).await;
        tasks.is_empty()
    });
    tokio::task::yield_now().await;
    assert!(!draining.is_finished());

    ticket.publish();

    assert!(tokio::time::timeout(HOST_RESPONSE_DRAIN_TEST_TIMEOUT, draining)
        .await
        .unwrap()
        .unwrap());
}

include!("host_response_drain_attachment.rs");
