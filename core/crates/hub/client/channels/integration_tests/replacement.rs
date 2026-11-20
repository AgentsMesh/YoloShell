use crate::{harness::Harness, peer::*};
use yolo_hub_rpc_protocol::{Command, Reply};

#[tokio::test]
async fn replacing_a_blocked_refresh_discards_its_stream_and_publishes_only_the_new_request() {
    let mut harness = Harness::start();
    let mut control = harness.accept().await;
    harness.ready().await;
    let previous = harness.refresh();
    let mut old = harness.accept().await;
    receive(&mut old).await;
    let current = harness.refresh();
    assert_ne!(previous, current);
    eof(&mut old).await;
    let mut catalog = harness.accept().await;
    let request = receive(&mut catalog).await;
    assert!(matches!(
        request.fields().command,
        Command::List { after: None, .. }
    ));
    reply(
        &mut catalog,
        &request,
        Reply::Terminals {
            terminals: vec![],
            next: None,
        },
    )
    .await;
    let event = tokio::time::timeout_at(deadline(), harness.catalogs.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(event.request_id, current);
    assert!(event.result.unwrap().terminals.is_empty());
    assert!(harness.catalogs.try_recv().is_err());
    harness.finish().await;
    eof(&mut control).await;
    eof(&mut catalog).await;
}
