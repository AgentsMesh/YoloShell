use crate::{commands, constants::CAPACITY, harness::Harness, liveness, operations, peer::*};
use yolo_hub_client_channels::{ControlError, ControlEvent, RequestScope};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::PublicError;

const IDLE_WINDOWS: usize = 5;

#[tokio::test]
async fn idle_probes_use_the_input_socket_and_preserve_its_generation() {
    let mut harness = Harness::start();
    let mut control = harness.accept().await;
    let generation = harness.ready().await;
    liveness::ready(&mut harness, &mut control, generation).await;
    let grant = commands::grant(harness.endpoint.hub);
    for _ in 0..IDLE_WINDOWS {
        liveness::idle().await;
        hello(&mut control, harness.endpoint.hub).await;
        operations::round_trip(
            &mut harness,
            &mut control,
            generation,
            commands::input(grant),
            Reply::Input { duplicate: false },
        )
        .await;
    }
    // A successful probe on another connection would not protect this stream.
    operations::cancel_without_replay(&mut harness).await;
    eof(&mut control).await;
}

#[tokio::test]
async fn failed_idle_probe_retires_only_its_generation_without_synthetic_input_results() {
    let mut harness = Harness::start();
    let mut control = harness.accept().await;
    let generation = harness.ready().await;
    liveness::ready(&mut harness, &mut control, generation).await;
    liveness::idle().await;
    assert!(matches!(
        receive(&mut control).await.fields().command,
        Command::Hello {}
    ));
    drop(control);
    assert!(matches!(harness.event().await,
        ControlEvent::Unreachable { generation: Some(previous), .. } if previous == generation
    ));
    let mut replacement = harness.accept().await;
    let next = harness.ready().await;
    assert_ne!(next, generation);
    harness.send(Some(generation), Command::Hello {}).await;
    assert!(matches!(
        harness.event().await,
        ControlEvent::Finished {
            result: Err(ControlError::StaleGeneration),
            connected: true,
            ..
        }
    ));
    let grant = commands::grant(harness.endpoint.hub);
    operations::round_trip(
        &mut harness,
        &mut replacement,
        next,
        commands::input(grant),
        Reply::Input { duplicate: false },
    )
    .await;
    harness.finish().await;
    eof(&mut replacement).await;
}

#[tokio::test]
async fn result_backpressure_does_not_suspend_the_actual_writer_socket_heartbeat() {
    let mut harness = Harness::start();
    let mut control = harness.accept().await;
    let generation = harness.ready().await;
    let grant = commands::grant(harness.endpoint.hub);
    for _ in 0..CAPACITY {
        harness.send(Some(generation), commands::input(grant)).await;
        let request = receive(&mut control).await;
        assert!(matches!(request.fields().command, Command::Input { .. }));
        reply(&mut control, &request, Reply::Input { duplicate: false }).await;
    }
    let scope = RequestScope::ordered_inputs();
    let validity = crate::ordered_inputs::validity(&scope);
    crate::scoped_requests::send(
        &harness,
        generation,
        commands::input(grant),
        validity.clone(),
    )
    .await;
    let rejected = receive(&mut control).await;
    reply(
        &mut control,
        &rejected,
        Reply::Error {
            code: PublicError::Conflict,
        },
    )
    .await;
    // The serial owner invalidates the scope before publishing the result.
    // This witnesses completion without draining the deliberately full UI queue.
    tokio::time::timeout_at(deadline(), async {
        while validity.is_current() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    liveness::idle().await;
    hello(&mut control, harness.endpoint.hub).await;
    for _ in 0..CAPACITY {
        assert!(matches!(
            harness.event().await,
            ControlEvent::Finished {
                result: Ok(Reply::Input { duplicate: false }),
                connected: true,
                ..
            }
        ));
    }
    assert!(matches!(
        harness.event().await,
        ControlEvent::Finished {
            result: Err(ControlError::Client(yolo_hub_client::ClientError::Remote(
                PublicError::Conflict
            ))),
            connected: true,
            ..
        }
    ));
    operations::round_trip(
        &mut harness,
        &mut control,
        generation,
        commands::input(grant),
        Reply::Input { duplicate: false },
    )
    .await;
    harness.finish().await;
    eof(&mut control).await;
}
