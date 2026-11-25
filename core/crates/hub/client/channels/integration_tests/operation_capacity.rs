use crate::{commands::grant, harness::Harness, operations::*, peer::*};
use yolo_hub_client_channels::{ControlError, ControlEvent, MAX_LIFECYCLE_CHANNELS};
use yolo_hub_rpc_protocol::{Command, Reply};

#[tokio::test]
async fn lifecycle_limit_rejects_without_opening_a_stream_and_reclaims_a_completed_slot() {
    let mut harness = Harness::start();
    let mut control = harness.accept().await;
    let generation = harness.ready().await;
    let mut pending = Vec::new();
    for _ in 0..MAX_LIFECYCLE_CHANNELS {
        pending.push(dispatched(&harness, generation).await);
    }
    let rejected = create();
    harness.send(Some(generation), rejected.clone()).await;
    let ControlEvent::Finished {
        request,
        result: Err(ControlError::OperationLimit),
        connected: true,
    } = harness.event().await
    else {
        panic!("the lifecycle limit must reject explicitly without disconnecting control")
    };
    assert_eq!(request.generation, Some(generation));
    assert_eq!(request.command, rejected);
    let grant = grant(harness.endpoint.hub);
    round_trip(
        &mut harness,
        &mut control,
        generation,
        Command::Renew { grant },
        Reply::Lease { grant },
    )
    .await;
    let (mut released, request) = pending.pop().unwrap();
    let result = created(harness.endpoint.hub);
    reply(&mut released, &request, result.clone()).await;
    completed(&mut harness, generation, &request.fields().command, result).await;
    eof(&mut released).await;
    // If the rejected operation had opened a socket, accepting this replacement
    // would expose its different request identity instead of the new command.
    pending.push(dispatched(&harness, generation).await);
    cancel_without_replay(&mut harness).await;
    eof(&mut control).await;
    for (mut operation, _) in pending {
        eof(&mut operation).await;
    }
}
