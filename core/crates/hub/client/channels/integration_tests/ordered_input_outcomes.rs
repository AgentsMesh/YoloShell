use crate::{commands::grant, harness::Harness, ordered_inputs::*, peer::*, scoped_requests::send};
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::{ControlError, ControlEvent, RequestScope};
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::PublicError;

#[tokio::test]
async fn an_unknown_prefix_outcome_cancels_suffix_without_poisoning_independent_control() {
    let mut harness = Harness::start();
    let mut peer = harness.accept().await;
    let generation = harness.ready().await;
    let grant = grant(harness.endpoint.hub);
    let scope = RequestScope::ordered_inputs();
    let prefix = input(grant, PREFIX);
    let suffix = input(grant, SUFFIX);
    send(&harness, generation, prefix.clone(), validity(&scope)).await;
    let dispatched = receive(&mut peer).await;
    assert_eq!(dispatched.fields().command, prefix);
    send(&harness, generation, suffix.clone(), validity(&scope)).await;
    reply(
        &mut peer,
        &dispatched,
        Reply::Error {
            code: PublicError::OutcomeUnknown,
        },
    )
    .await;
    wire_barrier(&harness, &mut peer, generation).await;
    let ControlEvent::Finished {
        request,
        result: Err(ControlError::Client(ClientError::Remote(PublicError::OutcomeUnknown))),
        connected: true,
    } = harness.event().await
    else {
        panic!()
    };
    assert_eq!(request.command, prefix);
    rejected(&mut harness, &suffix).await;
    completed_barrier(&mut harness).await;
    harness.finish().await;
    eof(&mut peer).await;
}

#[tokio::test]
async fn an_illegal_prefix_reply_cancels_suffix_across_connection_replacement_without_replay() {
    let mut harness = Harness::start();
    let mut peer = harness.accept().await;
    let generation = harness.ready().await;
    let grant = grant(harness.endpoint.hub);
    let scope = RequestScope::ordered_inputs();
    let prefix = input(grant, PREFIX);
    let suffix = input(grant, SUFFIX);
    send(&harness, generation, prefix.clone(), validity(&scope)).await;
    let dispatched = receive(&mut peer).await;
    send(&harness, generation, suffix.clone(), validity(&scope)).await;
    reply(&mut peer, &dispatched, Reply::Completed {}).await;
    // An actual EOF, not a timeout, proves no suffix escaped on the old stream.
    eof(&mut peer).await;
    crate::operations::generation_retired(&mut harness, generation).await;
    let ControlEvent::Finished {
        request,
        result: Err(ControlError::Client(error)),
        connected: false,
    } = harness.event().await
    else {
        panic!()
    };
    assert_eq!(request.command, prefix);
    assert!(error.outcome_is_unknown());
    assert!(!validity(&scope).is_current());
    let ControlEvent::Finished {
        request,
        result: Err(ControlError::NotConnected),
        connected: false,
    } = harness.event().await
    else {
        panic!()
    };
    assert_eq!(request.command, suffix);
    let mut replacement = harness.accept().await;
    let current = harness.ready().await;
    assert_ne!(current, generation);
    // Even an explicit new-generation attempt cannot reuse the failed scope.
    let later = input(grant, SUFFIX);
    send(&harness, current, later.clone(), validity(&scope)).await;
    wire_barrier(&harness, &mut replacement, current).await;
    rejected(&mut harness, &later).await;
    completed_barrier(&mut harness).await;
    harness.finish().await;
    eof(&mut replacement).await;
}
