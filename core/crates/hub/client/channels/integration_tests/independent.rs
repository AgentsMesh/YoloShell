use crate::{commands::*, harness::Harness, peer::*};
use yolo_hub_client_channels::ControlEvent;
use yolo_hub_rpc_protocol::{Command, Reply};

#[tokio::test]
async fn blocked_catalog_cannot_delay_input_renewal_release_or_shutdown() {
    let mut harness = Harness::start();
    let mut control = harness.accept().await;
    let generation = harness.ready().await;
    harness.refresh();
    let mut catalog = harness.accept().await;
    let blocked = receive(&mut catalog).await;
    assert!(matches!(blocked.fields().command, Command::List { .. }));
    let grant = grant(harness.endpoint.hub);
    let input = input(grant);
    harness.send(Some(generation), input.clone()).await;
    let request = receive(&mut control).await;
    assert_eq!(request.fields().command, input);
    reply(&mut control, &request, Reply::Input { duplicate: false }).await;
    assert!(matches!(
        harness.event().await,
        ControlEvent::Finished {
            result: Ok(Reply::Input { .. }),
            connected: true,
            ..
        }
    ));
    harness
        .send(Some(generation), Command::Renew { grant })
        .await;
    let request = receive(&mut control).await;
    assert_eq!(request.fields().command, Command::Renew { grant });
    reply(&mut control, &request, Reply::Lease { grant }).await;
    assert!(matches!(
        harness.event().await,
        ControlEvent::Finished {
            result: Ok(Reply::Lease { .. }),
            connected: true,
            ..
        }
    ));
    harness.controls.close();
    harness
        .send(Some(generation), Command::Release { grant })
        .await;
    harness.commands.take();
    let request = receive(&mut control).await;
    assert_eq!(request.fields().command, Command::Release { grant });
    reply(&mut control, &request, Reply::Completed {}).await;
    // The unanswered catalogue is still pending when shutdown begins.
    harness.finish().await;
    eof(&mut control).await;
    eof(&mut catalog).await;
}

#[tokio::test]
async fn cancelling_the_owner_closes_both_active_protocol_connections() {
    let mut harness = Harness::start();
    let mut control = harness.accept().await;
    harness.ready().await;
    harness.refresh();
    let mut catalog = harness.accept().await;
    receive(&mut catalog).await;
    harness.cancel().await;
    eof(&mut control).await;
    eof(&mut catalog).await;
}
