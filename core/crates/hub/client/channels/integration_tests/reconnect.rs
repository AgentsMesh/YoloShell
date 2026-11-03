use crate::{commands::*, harness::Harness, peer::*};
use yolo_hub_client_channels::{ControlError, ControlEvent};
use yolo_hub_rpc_protocol::Reply;

#[tokio::test]
async fn lost_reply_is_not_replayed_and_old_generation_cannot_use_a_reconnected_control() {
    let mut harness = Harness::start();
    let mut first = harness.accept().await;
    let old = harness.ready().await;
    let grant = grant(harness.endpoint.hub);
    let sent = input(grant);
    harness.send(Some(old), sent.clone()).await;
    assert_eq!(receive(&mut first).await.fields().command, sent);
    drop(first);
    crate::operations::generation_retired(&mut harness, old).await;
    let ControlEvent::Finished {
        result: Err(ControlError::Client(error)),
        connected: false,
        ..
    } = harness.event().await
    else {
        panic!()
    };
    assert!(error.outcome_is_unknown());
    let mut current = harness.accept().await;
    let fresh = harness.ready().await;
    assert_ne!(old, fresh);
    harness.send(Some(old), sent).await;
    assert!(matches!(
        harness.event().await,
        ControlEvent::Finished {
            result: Err(ControlError::StaleGeneration),
            connected: true,
            ..
        }
    ));
    let intended = input(grant);
    harness.send(Some(fresh), intended.clone()).await;
    let request = receive(&mut current).await;
    assert_eq!(request.fields().command, intended);
    reply(&mut current, &request, Reply::Input { duplicate: false }).await;
    assert!(matches!(
        harness.event().await,
        ControlEvent::Finished {
            result: Ok(Reply::Input { .. }),
            connected: true,
            ..
        }
    ));
    harness.finish().await;
    eof(&mut current).await;
}
