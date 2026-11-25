use crate::{commands::grant, harness::Harness, ordered_inputs::*, peer::*, scoped_requests::send};
use yolo_hub_client_channels::{ControlError, ControlEvent, ControlRequest, RequestScope};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::PublicError;

#[tokio::test]
async fn blocked_ordered_input_does_not_block_lease_renewal_and_unknown_still_cancels_suffix() {
    let mut harness = Harness::start();
    let mut writer = harness.accept().await;
    let generation = harness.ready().await;
    let grant = grant(harness.endpoint.hub);
    let scope = RequestScope::leased_inputs(std::time::Instant::now() + crate::constants::TIMEOUT);
    let prefix = input(grant, PREFIX);
    let suffix = input(grant, SUFFIX);
    send(&harness, generation, prefix.clone(), validity(&scope)).await;
    let blocked = receive(&mut writer).await;
    assert_eq!(blocked.fields().command, prefix);
    send(&harness, generation, suffix.clone(), validity(&scope)).await;
    harness
        .maintenance
        .as_ref()
        .unwrap()
        .send(ControlRequest {
            validity: None,
            generation: Some(generation),
            destination: harness.endpoint.hub,
            command: Command::Renew { grant },
        })
        .await
        .unwrap();
    let mut maintenance = harness.accept().await;
    let renewal = receive(&mut maintenance).await;
    assert_eq!(renewal.fields().command, Command::Renew { grant });
    reply(&mut maintenance, &renewal, Reply::Lease { grant }).await;
    let ControlEvent::Finished {
        request,
        result: Ok(Reply::Lease { .. }),
        connected: true,
    } = harness.event().await
    else {
        panic!()
    };
    assert_eq!(request.command, Command::Renew { grant });
    assert!(scope.renew_authority(std::time::Instant::now() + crate::constants::TIMEOUT));
    reply(
        &mut writer,
        &blocked,
        Reply::Error {
            code: PublicError::OutcomeUnknown,
        },
    )
    .await;
    let ControlEvent::Finished {
        request,
        result: Err(ControlError::Client(_)),
        connected: true,
    } = harness.event().await
    else {
        panic!()
    };
    assert_eq!(request.command, prefix);
    rejected(&mut harness, &suffix).await;
    assert!(!scope.renew_authority(std::time::Instant::now() + crate::constants::TIMEOUT));
    wire_barrier(&harness, &mut writer, generation).await;
    completed_barrier(&mut harness).await;
    harness.finish().await;
    eof(&mut writer).await;
    eof(&mut maintenance).await;
}

#[tokio::test]
async fn maintenance_rejects_stale_generation_before_opening_another_transport() {
    let mut harness = Harness::start();
    let mut writer = harness.accept().await;
    let generation = harness.ready().await;
    let grant = grant(harness.endpoint.hub);
    harness
        .maintenance
        .as_ref()
        .unwrap()
        .send(ControlRequest {
            validity: None,
            generation: Some(uuid::Uuid::new_v4()),
            destination: harness.endpoint.hub,
            command: Command::Renew { grant },
        })
        .await
        .unwrap();
    assert!(matches!(
        harness.event().await,
        ControlEvent::Finished {
            result: Err(ControlError::StaleGeneration),
            connected: true,
            ..
        }
    ));
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    assert!(harness.listener.poll_accept(&mut context).is_pending());
    wire_barrier(&harness, &mut writer, generation).await;
    completed_barrier(&mut harness).await;
    harness.finish().await;
    eof(&mut writer).await;
}
