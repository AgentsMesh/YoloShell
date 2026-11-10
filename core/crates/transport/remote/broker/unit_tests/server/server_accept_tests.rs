use super::super::constants::*;
use super::super::server::handle_listener_result;
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::net::UnixStream;

const ACCEPT_FAILURE: &str = "injected accept failure";

#[tokio::test(start_paused = true)]
async fn failed_listener_backoff_does_not_block_another_listener() {
    let slow = tokio::spawn(async {
        let mut backoff = crate::ingress::AcceptBackoff::new(
            ACCEPT_FAILURE_INITIAL_BACKOFF,
            ACCEPT_FAILURE_MAX_BACKOFF,
        );
        handle_listener_result(
            Err(io::Error::new(
                io::ErrorKind::ConnectionAborted,
                ACCEPT_FAILURE,
            )),
            &mut backoff,
            RUNTIME_LISTENER_LABEL,
            &mut |_: UnixStream| {},
        )
        .await;
    });
    tokio::task::yield_now().await;

    let accepted = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&accepted);
    let (server, _client) = UnixStream::pair().unwrap();
    let mut backoff = crate::ingress::AcceptBackoff::new(
        ACCEPT_FAILURE_INITIAL_BACKOFF,
        ACCEPT_FAILURE_MAX_BACKOFF,
    );
    handle_listener_result(
        Ok(server),
        &mut backoff,
        MANAGEMENT_LISTENER_LABEL,
        &mut |_| observed.store(true, Ordering::Release),
    )
    .await;

    assert!(accepted.load(Ordering::Acquire));
    assert!(!slow.is_finished());
    tokio::time::advance(ACCEPT_FAILURE_INITIAL_BACKOFF).await;
    slow.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn listener_continues_after_an_accept_failure() {
    let continued = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&continued);
    let task = tokio::spawn(async move {
        let mut backoff = crate::ingress::AcceptBackoff::new(
            ACCEPT_FAILURE_INITIAL_BACKOFF,
            ACCEPT_FAILURE_MAX_BACKOFF,
        );
        let mut on_stream = |_: UnixStream| observed.store(true, Ordering::Release);
        handle_listener_result(
            Err(io::Error::other(ACCEPT_FAILURE)),
            &mut backoff,
            RUNTIME_LISTENER_LABEL,
            &mut on_stream,
        )
        .await;
        let (server, _client) = UnixStream::pair().unwrap();
        handle_listener_result(
            Ok(server),
            &mut backoff,
            RUNTIME_LISTENER_LABEL,
            &mut on_stream,
        )
        .await;
    });
    tokio::task::yield_now().await;
    tokio::time::advance(ACCEPT_FAILURE_INITIAL_BACKOFF).await;
    task.await.unwrap();
    assert!(continued.load(Ordering::Acquire));
}
