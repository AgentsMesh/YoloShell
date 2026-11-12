use crate::{harness::Harness, operations, peer::*, queries, query_requests::*};
use yolo_hub_client_channels::{ControlError, MAX_LIFECYCLE_CHANNELS, MAX_QUERY_CHANNELS};
use yolo_hub_rpc_protocol::{Command, Reply};

#[tokio::test]
async fn query_and_lifecycle_capacity_are_separate_and_completed_query_slots_are_reclaimed() {
    let mut harness = Harness::start();
    let mut control = harness.accept().await;
    let generation = harness.ready().await;
    let mut pending_queries = Vec::new();
    for _ in 0..MAX_QUERY_CHANNELS {
        pending_queries.push(queries::dispatched(&harness, generation).await);
    }
    let rejected = query();
    harness.send(Some(generation), rejected.clone()).await;
    assert!(matches!(
        queries::failure(&mut harness, generation, &rejected, true)
            .await
            .1,
        ControlError::QueryLimit
    ));
    // Query saturation does not use a single Create/Finalize slot.
    let mut lifecycles = Vec::new();
    let Reply::Created { creation } = operations::created(harness.endpoint.hub) else {
        panic!()
    };
    let finalize = Command::Finalize {
        creation_token: uuid::Uuid::new_v4(),
        creation,
    };
    harness.send(Some(generation), finalize.clone()).await;
    let mut finalizer = harness.accept().await;
    let request = receive(&mut finalizer).await;
    assert_eq!(request.fields().command, finalize);
    lifecycles.push((finalizer, request));
    for _ in lifecycles.len()..MAX_LIFECYCLE_CHANNELS {
        lifecycles.push(operations::dispatched(&harness, generation).await);
    }
    let rejected = operations::create();
    harness.send(Some(generation), rejected.clone()).await;
    assert!(matches!(
        queries::failure(&mut harness, generation, &rejected, true)
            .await
            .1,
        ControlError::OperationLimit
    ));
    let (mut completed, request) = pending_queries.pop().unwrap();
    let result = ready(&request.fields().command, harness.endpoint.hub);
    reply(&mut completed, &request, result.clone()).await;
    operations::completed(&mut harness, generation, &request.fields().command, result).await;
    eof(&mut completed).await;
    // Lifecycle saturation does not block a new query. Its identity also proves
    // the earlier rejected query never opened a socket or entered a retry queue.
    pending_queries.push(queries::dispatched(&harness, generation).await);
    let hello = hello_reply(harness.endpoint.hub);
    operations::round_trip(
        &mut harness,
        &mut control,
        generation,
        Command::Hello {},
        hello,
    )
    .await;
    harness.finish().await;
    eof(&mut control).await;
    for (mut peer, _) in pending_queries.into_iter().chain(lifecycles) {
        eof(&mut peer).await;
    }
}
