use crate::{commands::*, harness::Harness, operations, peer::*, queries, query_requests::*};
use yolo_hub_client_channels::ControlError;
use yolo_hub_rpc_protocol::{Command, Reply};

#[tokio::test]
async fn blocked_query_uses_an_independent_socket_and_does_not_block_writer_or_lifecycle() {
    let mut harness = Harness::start();
    let mut control = harness.accept().await;
    let generation = harness.ready().await;
    let (mut query_peer, query_request) = queries::dispatched(&harness, generation).await;
    let writer = grant(harness.endpoint.hub);
    let hello = hello_reply(harness.endpoint.hub);
    for (command, response) in [
        (input(writer), Reply::Input { duplicate: false }),
        (Command::Hello {}, hello),
        (
            Command::Renew { grant: writer },
            Reply::Lease { grant: writer },
        ),
    ] {
        operations::round_trip(&mut harness, &mut control, generation, command, response).await;
    }
    let (mut lifecycle, creation) = operations::dispatched(&harness, generation).await;
    let created = operations::created(harness.endpoint.hub);
    reply(&mut lifecycle, &creation, created.clone()).await;
    operations::completed(
        &mut harness,
        generation,
        &creation.fields().command,
        created,
    )
    .await;
    let result = ready(&query_request.fields().command, harness.endpoint.hub);
    reply(&mut query_peer, &query_request, result.clone()).await;
    operations::completed(
        &mut harness,
        generation,
        &query_request.fields().command,
        result,
    )
    .await;
    harness.finish().await;
    eof(&mut control).await;
    eof(&mut query_peer).await;
    eof(&mut lifecycle).await;
}

#[tokio::test]
async fn query_dial_handshake_and_response_errors_preserve_the_healthy_fast_connection() {
    for stage in [
        FailureStage::Dial,
        FailureStage::Handshake,
        FailureStage::Response,
    ] {
        let mut harness = Harness::start();
        let mut control = harness.accept().await;
        let generation = harness.ready().await;
        let command = query();
        if matches!(stage, FailureStage::Dial) {
            std::fs::remove_file(harness.endpoint.entry.local_socket().unwrap()).unwrap();
        }
        harness.send(Some(generation), command.clone()).await;
        match stage {
            FailureStage::Dial => {}
            FailureStage::Handshake => drop(operations::awaiting_handshake(&harness).await),
            FailureStage::Response => {
                let mut query_peer = harness.accept().await;
                assert_eq!(receive(&mut query_peer).await.fields().command, command);
                drop(query_peer);
            }
        }
        let (_, error) = queries::failure(&mut harness, generation, &command, true).await;
        let ControlError::Client(error) = error else {
            panic!()
        };
        assert!(!error.outcome_is_unknown());
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
    }
}

enum FailureStage {
    Dial,
    Handshake,
    Response,
}
