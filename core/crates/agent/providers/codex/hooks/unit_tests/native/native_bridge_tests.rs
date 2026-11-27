use super::test_support::{Fixture, request};
use crate::codex_hooks::{
    CodexNativeAck, CodexNativeAckCode, CodexNativeSpool, read_native_request, write_native_ack,
};
use tokio::net::UnixListener;
use tokio::time::Instant;

const SOCKET_NAME: &str = "native.sock";
const TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);
const TRANSIENT_REJECTIONS: usize = 3;

#[tokio::test]
async fn temporary_process_unavailability_uses_the_full_handshake_budget() {
    let fixture = Fixture::new();
    let callback = request(&fixture);
    let socket = fixture.paths.home().join(SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let expected = callback.clone();
    let server = tokio::spawn(async move {
        for _ in 0..TRANSIENT_REJECTIONS {
            let (mut stream, _) = listener.accept().await.unwrap();
            let request = read_native_request(&mut stream).await.unwrap();
            assert_eq!(request, expected);
            write_native_ack(
                &mut stream,
                &CodexNativeAck::rejected(&request, CodexNativeAckCode::ProcessUnavailable),
            )
            .await
            .unwrap();
        }
        let (mut stream, _) = listener.accept().await.unwrap();
        let retried = read_native_request(&mut stream).await.unwrap();
        assert_eq!(retried, expected);
        write_native_ack(&mut stream, &CodexNativeAck::accepted(&retried, false))
            .await
            .unwrap();
    });
    super::bridge::deliver(&socket, &callback, Instant::now() + TEST_TIMEOUT)
        .await
        .unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn acknowledgement_loss_retries_the_same_persisted_event() {
    let fixture = Fixture::new();
    let callback = request(&fixture);
    let spool = CodexNativeSpool::for_scope(
        &fixture.paths,
        callback.terminal_id,
        callback.runtime_epoch,
    )
    .unwrap();
    let occurrence = spool.persist(&callback).unwrap();
    let socket = fixture.paths.home().join(SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let expected = callback.clone();
    let server = tokio::spawn(async move {
        let (mut first, _) = listener.accept().await.unwrap();
        assert_eq!(read_native_request(&mut first).await.unwrap(), expected);
        drop(first);
        let (mut second, _) = listener.accept().await.unwrap();
        let replay = read_native_request(&mut second).await.unwrap();
        assert_eq!(replay, expected);
        write_native_ack(&mut second, &CodexNativeAck::accepted(&replay, true))
            .await
            .unwrap();
    });
    super::bridge::deliver(&socket, &callback, Instant::now() + TEST_TIMEOUT)
        .await
        .unwrap();
    server.await.unwrap();
    spool.verify(&occurrence).unwrap();
    spool.remove(&occurrence).unwrap();
}

#[tokio::test]
async fn invalid_ack_retains_the_durable_occurrence() {
    let fixture = Fixture::new();
    let callback = request(&fixture);
    let spool = CodexNativeSpool::for_scope(
        &fixture.paths,
        callback.terminal_id,
        callback.runtime_epoch,
    )
    .unwrap();
    let occurrence = spool.persist(&callback).unwrap();
    let socket = fixture.paths.home().join(SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let request = read_native_request(&mut stream).await.unwrap();
        let mut ack = CodexNativeAck::accepted(&request, false);
        ack.event_id = uuid::Uuid::new_v4();
        write_native_ack(&mut stream, &ack).await.unwrap();
    });
    assert!(
        super::bridge::deliver(&socket, &callback, Instant::now() + TEST_TIMEOUT)
            .await
            .is_err()
    );
    server.await.unwrap();
    spool.verify(&occurrence).unwrap();
}

#[tokio::test]
async fn terminal_rejection_is_not_retried() {
    let fixture = Fixture::new();
    let callback = request(&fixture);
    let socket = fixture.paths.home().join(SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let request = read_native_request(&mut stream).await.unwrap();
        write_native_ack(
            &mut stream,
            &CodexNativeAck::rejected(&request, CodexNativeAckCode::AuthorityRevoked),
        )
        .await
        .unwrap();
    });
    assert!(
        super::bridge::deliver(&socket, &callback, Instant::now() + TEST_TIMEOUT)
            .await
            .is_err()
    );
    server.await.unwrap();
}
