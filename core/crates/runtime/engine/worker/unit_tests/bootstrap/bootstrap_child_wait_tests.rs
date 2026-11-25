use super::*;

const PREPARED_WAIT_ENDED_DETAIL: &str =
    "prepared bootstrap child stopped waiting without a gate decision";

#[tokio::test(start_paused = true)]
async fn prepared_child_wait_has_no_independent_deadline() {
    let (mut child, mut worker) = LocalUnixStream::pair().unwrap();
    let wait = tokio::spawn(async move { await_release(&mut child).await });
    tokio::task::yield_now().await;

    tokio::time::advance(BOOTSTRAP_GATE_TIMEOUT.saturating_add(BOOTSTRAP_GATE_TIMEOUT)).await;
    tokio::task::yield_now().await;
    assert!(!wait.is_finished(), "{PREPARED_WAIT_ENDED_DETAIL}");

    write_frame(&mut worker, &BootstrapGateReply::Go)
        .await
        .unwrap();
    wait.await.unwrap().unwrap();
}

#[tokio::test]
async fn prepared_child_wait_fails_closed_on_worker_disconnect() {
    let (mut child, worker) = LocalUnixStream::pair().unwrap();
    drop(worker);
    let error = await_release(&mut child).await.unwrap_err();
    assert!(
        error
            .to_string()
            .contains(BOOTSTRAP_GATE_RELEASE_READ_CONTEXT)
    );
}
