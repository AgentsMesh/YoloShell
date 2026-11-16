use crate::{commands, harness::Harness, liveness, maintenance, operations, peer::*};
use yolo_hub_rpc_protocol::{Command, Reply};

#[tokio::test]
async fn idle_maintenance_connection_is_retired_before_the_next_renewal() {
    let mut harness = Harness::start();
    let mut control = harness.accept().await;
    let generation = harness.ready().await;
    liveness::ready(&mut harness, &mut control, generation).await;
    let grant = commands::grant(harness.endpoint.hub);
    let renewal = Command::Renew { grant };
    maintenance::send(&harness, generation, renewal.clone(), None).await;
    let mut cached = harness.accept().await;
    maintenance::renew(&mut cached, grant).await;
    operations::completed(&mut harness, generation, &renewal, Reply::Lease { grant }).await;
    liveness::idle().await;
    hello(&mut control, harness.endpoint.hub).await;
    eof(&mut cached).await;
    maintenance::send(&harness, generation, renewal.clone(), None).await;
    let mut replacement = harness.accept().await;
    maintenance::renew(&mut replacement, grant).await;
    operations::completed(&mut harness, generation, &renewal, Reply::Lease { grant }).await;
    harness.finish().await;
    eof(&mut replacement).await;
    eof(&mut control).await;
}

#[tokio::test]
async fn idle_catalogue_connection_is_retired_before_the_next_refresh() {
    let mut harness = Harness::start();
    let mut control = harness.accept().await;
    let generation = harness.ready().await;
    liveness::ready(&mut harness, &mut control, generation).await;
    let first = harness.refresh();
    let mut cached = harness.accept().await;
    empty(&mut cached).await;
    assert_eq!(harness.catalogs.recv().await.unwrap().request_id, first);
    liveness::idle().await;
    hello(&mut control, harness.endpoint.hub).await;
    eof(&mut cached).await;
    let second = harness.refresh();
    let mut replacement = harness.accept().await;
    empty(&mut replacement).await;
    let refreshed = harness.catalogs.recv().await.unwrap();
    assert_eq!(refreshed.request_id, second);
    assert!(refreshed.result.is_ok());
    harness.finish().await;
    eof(&mut replacement).await;
    eof(&mut control).await;
}

async fn empty(peer: &mut Peer) {
    let request = receive(peer).await;
    assert!(matches!(request.fields().command, Command::List { .. }));
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
