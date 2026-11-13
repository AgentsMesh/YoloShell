use crate::{
    commands::{grant, input},
    harness::Harness,
    maintenance, operations,
    peer::*,
};
use yolo_hub_client_channels::{ControlError, ControlEvent, RequestScope};
use yolo_hub_rpc_protocol::{Command, Reply};

#[tokio::test]
async fn maintenance_rejects_input_and_retired_renewal_without_opening_another_transport() {
    let mut harness = Harness::start();
    let mut control = harness.accept().await;
    let generation = harness.ready().await;
    let grant = grant(harness.endpoint.hub);
    let scope = RequestScope::default();
    let retired = scope.validity_until(deadline().into_std());
    scope.retire();
    for (command, validity) in [
        (input(grant), None),
        (Command::Renew { grant }, Some(retired)),
    ] {
        maintenance::send(&harness, generation, command.clone(), validity).await;
        let ControlEvent::Finished {
            request,
            result: Err(ControlError::Invalidated),
            connected: true,
        } = harness.event().await
        else {
            panic!("rejected maintenance must remain known not sent")
        };
        assert_eq!(request.command, command);
        assert_eq!(request.generation, Some(generation));
    }
    let current = RequestScope::default();
    let renewal = Command::Renew { grant };
    maintenance::send(
        &harness,
        generation,
        renewal.clone(),
        Some(current.validity_until(deadline().into_std())),
    )
    .await;
    let mut peer = harness.accept().await;
    maintenance::renew(&mut peer, grant).await;
    operations::completed(&mut harness, generation, &renewal, Reply::Lease { grant }).await;
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    assert!(harness.listener.poll_accept(&mut context).is_pending());
    harness.finish().await;
    eof(&mut peer).await;
    eof(&mut control).await;
}

#[tokio::test]
async fn view_retirement_during_maintenance_handshake_does_not_renew_retired_authority() {
    let mut harness = Harness::start();
    let mut control = harness.accept().await;
    let generation = harness.ready().await;
    let grant = grant(harness.endpoint.hub);
    let renewal = Command::Renew { grant };
    let scope = RequestScope::default();
    let validity = scope.validity_until(deadline().into_std());
    maintenance::send(
        &harness,
        generation,
        renewal.clone(),
        Some(validity.clone()),
    )
    .await;
    let (mut peer, opening) = operations::awaiting_handshake(&harness).await;
    scope.retire();
    reply(&mut peer, &opening, hello_reply(harness.endpoint.hub)).await;
    let ControlEvent::Finished {
        request,
        result: Err(ControlError::Invalidated),
        connected: true,
    } = harness.event().await
    else {
        panic!("retirement must be checked after handshake")
    };
    assert_eq!(request.command, renewal);
    assert!(request.validity.unwrap().same_scope(&validity));
    let another_terminal = crate::commands::grant(harness.endpoint.hub);
    let intended = Command::Renew {
        grant: another_terminal,
    };
    maintenance::send(&harness, generation, intended.clone(), None).await;
    maintenance::renew(&mut peer, another_terminal).await;
    operations::completed(
        &mut harness,
        generation,
        &intended,
        Reply::Lease {
            grant: another_terminal,
        },
    )
    .await;
    harness.finish().await;
    eof(&mut peer).await;
    eof(&mut control).await;
}
