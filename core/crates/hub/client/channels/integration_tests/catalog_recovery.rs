use crate::{harness::Harness, peer::*};
use yolo_hub_rpc_protocol::Reply;

async fn empty(peer: &mut Peer) {
    let request = receive(peer).await;
    reply(
        peer,
        &request,
        Reply::Terminals {
            terminals: vec![],
            next: None,
        },
    )
    .await;
}
async fn loaded(harness: &mut Harness, id: uuid::Uuid) {
    let event = tokio::time::timeout_at(deadline(), harness.catalogs.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(event.request_id, id);
    assert!(event.result.is_ok());
}

#[tokio::test]
async fn completed_catalogue_connection_is_reused_but_a_malformed_one_is_retired() {
    let mut harness = Harness::start();
    let mut control = harness.accept().await;
    harness.ready().await;
    let first = harness.refresh();
    let mut catalog = harness.accept().await;
    empty(&mut catalog).await;
    loaded(&mut harness, first).await;
    let second = harness.refresh();
    empty(&mut catalog).await;
    loaded(&mut harness, second).await;
    let failed = harness.refresh();
    let request = receive(&mut catalog).await;
    reply(&mut catalog, &request, Reply::Completed {}).await;
    let failure = tokio::time::timeout_at(deadline(), harness.catalogs.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(failure.request_id, failed);
    assert!(failure.result.is_err());
    eof(&mut catalog).await;
    let recovered = harness.refresh();
    let mut replacement = harness.accept().await;
    empty(&mut replacement).await;
    loaded(&mut harness, recovered).await;
    assert!(harness.controls.try_recv().is_err());
    harness.finish().await;
    eof(&mut replacement).await;
    eof(&mut control).await;
}

#[tokio::test]
async fn clearing_refresh_and_closing_its_consumer_cancel_only_the_catalogue() {
    let mut harness = Harness::start();
    let mut control = harness.accept().await;
    harness.ready().await;
    harness.refresh();
    let mut old = harness.accept().await;
    receive(&mut old).await;
    harness.refresh.as_ref().unwrap().send_replace(None);
    eof(&mut old).await;
    harness.refresh();
    let mut replacement = harness.accept().await;
    receive(&mut replacement).await;
    harness.catalogs.close();
    eof(&mut replacement).await;
    harness.finish().await;
    eof(&mut control).await;
}
