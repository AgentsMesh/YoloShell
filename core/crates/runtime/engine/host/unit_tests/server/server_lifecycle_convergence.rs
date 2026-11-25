use crate::host::tests::*;
use super::*;

const HOST_INGRESS_DROP_NOT_OBSERVED: &str = "HostIngress Drop did not signal shutdown";
const HOST_INGRESS_DETACHED_TASK_FINISHED_EARLY: &str =
    "detached HostIngress task finished before release";
const HOST_INGRESS_DETACHED_TASK_NOT_FINISHED: &str =
    "detached HostIngress task did not finish";
const SERVE_START_NOT_OBSERVED: &str = "host serve startup was not observed";
const CANCELED_SERVE_DID_NOT_CANCEL: &str = "aborted host serve task did not report cancellation";
const CANCELED_SERVE_DID_NOT_FINISH: &str = "aborted host serve task did not finish";

#[tokio::test]
async fn host_ingress_drop_signals_detached_task_to_finish() {
    let (shutdown, receiver) = tokio::sync::oneshot::channel();
    let (observed, observation) = tokio::sync::oneshot::channel();
    let (release, released) = tokio::sync::oneshot::channel();
    let (finished, completion) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        let _ = receiver.await;
        let _ = observed.send(());
        let _ = released.await;
        let _ = finished.send(());
        Ok(())
    });
    let ingress = HostIngress {
        task,
        shutdown: Some(shutdown),
    };

    drop(ingress);

    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, observation)
        .await
        .expect(HOST_INGRESS_DROP_NOT_OBSERVED)
        .expect(HOST_INGRESS_DROP_NOT_OBSERVED);
    assert!(
        !completion.is_terminated(),
        "{HOST_INGRESS_DETACHED_TASK_FINISHED_EARLY}"
    );
    release.send(()).unwrap();
    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, completion)
        .await
        .expect(HOST_INGRESS_DETACHED_TASK_NOT_FINISHED)
        .expect(HOST_INGRESS_DETACHED_TASK_NOT_FINISHED);
}

#[tokio::test(flavor = "multi_thread")]
async fn canceled_outer_serve_eventually_releases_same_home() {
    let directory = tempdir().unwrap();
    let home = directory.path().join(HOST_SHUTDOWN_HOME_NAME);
    let host = Host::open(HostConfig::new(&home)).unwrap();
    let socket = host.socket_path().to_path_buf();
    let (started, startup) = tokio::sync::oneshot::channel();
    let serving = tokio::spawn(host.serve_until_observed(std::future::pending(), started));
    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, startup)
        .await
        .expect(SERVE_START_NOT_OBSERVED)
        .expect(SERVE_START_NOT_OBSERVED);
    assert!(socket.exists());

    serving.abort();
    let joined = tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, serving)
        .await
        .expect(CANCELED_SERVE_DID_NOT_FINISH);
    assert!(
        matches!(joined, Err(error) if error.is_cancelled()),
        "{CANCELED_SERVE_DID_NOT_CANCEL}"
    );
    let replacement = wait_for_replacement_host(&home).await;
    assert!(!socket.exists());
    drop(replacement);
}
