use super::{constants::*, fixture};
use crate::command_io;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::watch,
    time::timeout,
};
use yolo_hub_relay_control_protocol::{
    Outcome, Request, Response, decode_response, encode_request,
};

#[tokio::test]
async fn closing_the_ui_channel_cancels_the_reply_receiver_before_queued_dispatch() {
    let request = fixture::request();
    let (io, mut incoming, mut peer) = fixture::channel(&request);
    let (cancel, cancelled) = watch::channel(false);
    let observed = cancelled.clone();
    let task = tokio::spawn(command_io::run(Some(io), cancel, cancelled));
    send(&mut peer, &request).await;
    let queued = timeout(TEST_TIMEOUT, incoming.incoming.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(!queued.response.is_closed());
    drop(peer);
    assert!(timeout(TEST_TIMEOUT, task).await.unwrap().unwrap().is_err());
    assert!(queued.response.is_closed());
    assert!(*observed.borrow());
}

#[tokio::test]
async fn command_channel_returns_only_the_exact_correlated_response_and_can_retire() {
    let request = fixture::request();
    let (io, mut incoming, mut peer) = fixture::channel(&request);
    let (cancel, cancelled) = watch::channel(false);
    let task = tokio::spawn(command_io::run(Some(io), cancel.clone(), cancelled));
    send(&mut peer, &request).await;
    let queued = timeout(TEST_TIMEOUT, incoming.incoming.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(queued.request, request);
    queued
        .response
        .send(Response::new(&request, Outcome::Unknown {}).unwrap())
        .unwrap();
    let response = receive(&mut peer).await;
    response.validate_for(&request).unwrap();
    assert_eq!(response.fields().outcome, Outcome::Unknown {});
    cancel.send(true).unwrap();
    timeout(TEST_TIMEOUT, task).await.unwrap().unwrap().unwrap();
    assert!(incoming.incoming.recv().await.is_none());
}

#[tokio::test]
async fn a_reply_for_another_request_closes_the_channel_without_publishing_it() {
    let request = fixture::request();
    let (io, mut incoming, mut peer) = fixture::channel(&request);
    let (cancel, cancelled) = watch::channel(false);
    let task = tokio::spawn(command_io::run(Some(io), cancel, cancelled));
    send(&mut peer, &request).await;
    let queued = timeout(TEST_TIMEOUT, incoming.incoming.recv())
        .await
        .unwrap()
        .unwrap();
    queued
        .response
        .send(Response::new(&fixture::request(), Outcome::Unknown {}).unwrap())
        .unwrap();
    assert!(timeout(TEST_TIMEOUT, task).await.unwrap().unwrap().is_err());
    let mut byte = [0u8];
    assert_eq!(peer.read(&mut byte).await.unwrap(), usize::default());
}

#[tokio::test]
async fn pipelined_commands_retire_unfinished_work_and_are_not_replayed() {
    let request = fixture::request();
    let (io, mut incoming, mut peer) = fixture::channel(&request);
    let (cancel, cancelled) = watch::channel(false);
    let task = tokio::spawn(command_io::run(Some(io), cancel, cancelled));
    send(&mut peer, &request).await;
    let queued = timeout(TEST_TIMEOUT, incoming.incoming.recv())
        .await
        .unwrap()
        .unwrap();
    send(&mut peer, &request).await;
    assert!(timeout(TEST_TIMEOUT, task).await.unwrap().unwrap().is_err());
    assert!(queued.response.is_closed());
    assert!(incoming.incoming.recv().await.is_none());
}

async fn send(peer: &mut tokio::net::UnixStream, request: &Request) {
    let bytes = encode_request(request).unwrap();
    peer.write_all(&(bytes.len() as u32).to_be_bytes()).await.unwrap();
    peer.write_all(&bytes).await.unwrap();
}

async fn receive(peer: &mut tokio::net::UnixStream) -> Response {
    timeout(TEST_TIMEOUT, async {
        let length = peer.read_u32().await.unwrap();
        let mut bytes = vec![0; length as usize];
        peer.read_exact(&mut bytes).await.unwrap();
        decode_response(&bytes).unwrap()
    })
    .await
    .unwrap()
}
