use crate::{commands::grant, harness::Harness, maintenance, operations, peer::*};
use yolo_hub_client_channels::{ControlError, ControlEvent};
use yolo_hub_rpc_protocol::{Command, Reply};

#[tokio::test]
async fn renewal_reuses_healthy_transport_and_failed_renewal_is_never_replayed() {
    let mut harness = Harness::start();
    let mut control = harness.accept().await;
    let generation = harness.ready().await;
    let original = grant(harness.endpoint.hub);
    let renewal = Command::Renew { grant: original };
    maintenance::send(&harness, generation, renewal.clone(), None).await;
    let mut peer = harness.accept().await;
    maintenance::renew(&mut peer, original).await;
    operations::completed(
        &mut harness,
        generation,
        &renewal,
        Reply::Lease { grant: original },
    )
    .await;
    maintenance::send(&harness, generation, renewal.clone(), None).await;
    maintenance::renew(&mut peer, original).await;
    operations::completed(
        &mut harness,
        generation,
        &renewal,
        Reply::Lease { grant: original },
    )
    .await;
    maintenance::send(&harness, generation, renewal.clone(), None).await;
    assert_eq!(receive(&mut peer).await.fields().command, renewal);
    drop(peer);
    let ControlEvent::Finished {
        request,
        result: Err(ControlError::Client(error)),
        connected: true,
    } = harness.event().await
    else {
        panic!("renewal failure must preserve the independent control lane")
    };
    assert_eq!(request.command, renewal);
    assert_eq!(request.generation, Some(generation));
    assert!(error.outcome_is_unknown());
    let other_terminal = grant(harness.endpoint.hub);
    let intended = Command::Renew {
        grant: other_terminal,
    };
    maintenance::send(&harness, generation, intended.clone(), None).await;
    let mut replacement = harness.accept().await;
    maintenance::renew(&mut replacement, other_terminal).await;
    operations::completed(
        &mut harness,
        generation,
        &intended,
        Reply::Lease {
            grant: other_terminal,
        },
    )
    .await;
    harness.finish().await;
    eof(&mut replacement).await;
    eof(&mut control).await;
}

#[tokio::test]
async fn closing_event_consumer_releases_maintenance_transport_after_pending_result() {
    let mut harness = Harness::start();
    let mut control = harness.accept().await;
    let generation = harness.ready().await;
    let grant = grant(harness.endpoint.hub);
    maintenance::send(&harness, generation, Command::Renew { grant }, None).await;
    let mut peer = harness.accept().await;
    let request = receive(&mut peer).await;
    assert_eq!(request.fields().command, Command::Renew { grant });
    harness.controls.close();
    reply(&mut peer, &request, Reply::Lease { grant }).await;
    eof(&mut peer).await;
    assert!(harness.maintenance.as_ref().unwrap().is_closed());
    harness.finish().await;
    eof(&mut control).await;
}
