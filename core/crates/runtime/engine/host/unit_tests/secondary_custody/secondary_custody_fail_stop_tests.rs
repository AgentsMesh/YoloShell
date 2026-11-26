use super::*;
use std::time::Duration;

const FIRST_FAIL_STOP_DETAIL: &str = "first exact custody failure";
const SECOND_FAIL_STOP_DETAIL: &str = "later custody failure must not replace the first";
const SERVE_FAIL_STOP_DETAIL: &str = "established custody stream became indeterminate";
const FAIL_STOP_TEST_TIMEOUT: Duration = Duration::from_secs(10);

#[tokio::test]
async fn fail_stop_rejects_admission_and_preserves_the_first_diagnostic() {
    let drain = crate::host::request_drain::HostRequestDrain::default();

    drain.fail_stop(FIRST_FAIL_STOP_DETAIL);
    drain.fail_stop(SECOND_FAIL_STOP_DETAIL);

    assert!(drain.is_draining());
    assert_eq!(
        drain.admit_mutation().await.err().unwrap().message,
        FIRST_FAIL_STOP_DETAIL
    );
    assert_eq!(
        drain.ensure_attachment_open().unwrap_err().message,
        FIRST_FAIL_STOP_DETAIL
    );
    assert_eq!(
        drain.wait_for_shutdown().await.unwrap_err(),
        FIRST_FAIL_STOP_DETAIL
    );
}

#[tokio::test]
async fn reverting_an_idle_drain_cannot_reopen_a_concurrent_fail_stop() {
    let drain = crate::host::request_drain::HostRequestDrain::default();
    let preparation = drain.prepare().await.unwrap();

    drain.fail_stop(FIRST_FAIL_STOP_DETAIL);
    drop(preparation);

    assert!(drain.is_draining());
    assert!(drain.admit_mutation().await.is_err());
    assert_eq!(
        drain.wait_for_shutdown().await.unwrap_err(),
        FIRST_FAIL_STOP_DETAIL
    );
}

#[tokio::test]
async fn fail_stop_takes_precedence_over_an_already_committed_idle_shutdown() {
    let drain = crate::host::request_drain::HostRequestDrain::default();
    drain.prepare().await.unwrap().commit();

    drain.fail_stop(FIRST_FAIL_STOP_DETAIL);

    assert_eq!(
        drain.wait_for_shutdown().await.unwrap_err(),
        FIRST_FAIL_STOP_DETAIL
    );
}

#[tokio::test]
async fn serve_until_tears_down_ingress_and_returns_the_fail_stop_diagnostic() {
    let directory = tempdir().unwrap();
    let host = Host::open(crate::host::HostConfig::new(directory.path())).unwrap();
    let drain = host.request_drain.clone();
    let socket = host.socket_path().to_path_buf();
    let serving = tokio::spawn(host.serve_until(std::future::pending(), |_| Ok(())));
    wait_for_socket(&socket).await;

    drain.fail_stop(SERVE_FAIL_STOP_DETAIL);
    let error = tokio::time::timeout(FAIL_STOP_TEST_TIMEOUT, serving)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();

    assert!(format!("{error:#}").contains(SERVE_FAIL_STOP_DETAIL));
    assert!(!socket.exists());
}

async fn wait_for_socket(socket: &std::path::Path) {
    tokio::time::timeout(FAIL_STOP_TEST_TIMEOUT, async {
        while !socket.exists() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}
