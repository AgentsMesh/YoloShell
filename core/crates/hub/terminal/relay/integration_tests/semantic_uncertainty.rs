use super::{agent::*, command_peer::*, constants::*, fixtures::*, peer::*, settings::*};
use tokio::{
    io::{self, AsyncWriteExt},
    sync::{mpsc, oneshot, watch},
    time::timeout_at,
};
use yolo_hub_relay_control_protocol::{NotSentReason, Outcome as ControlOutcome};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_relay::{
    ControlIntent, Outcome, STATUS_QUEUE_CAPACITY, Status, run_with_commands,
};

#[tokio::test]
async fn lost_or_mismatched_receipt_returns_unknown_and_cleanup_never_replays_interrupt() {
    for malformed in [false, true] {
        let (_directory, socket, listener) = listener();
        let (input, mut input_peer) = io::duplex(PIPE_CAPACITY);
        let (_resize, sizes) = watch::channel(None);
        let (status, mut statuses) = mpsc::channel(STATUS_QUEUE_CAPACITY);
        let (send, commands) = channel();
        let grant = grant();
        let server = async {
            let (mut control, mut terminal) = acquiring(&listener, grant).await;
            assert_eq!(statuses.recv().await, Some(Status::ControlAcquired));
            let intent = interrupt();
            let response = enqueue(&send, intent.clone()).await;
            let request = receive(&mut control).await;
            assert_eq!(
                request.fields().command,
                Command::AgentControl {
                    request: intent.fields().request.clone(),
                    grant
                }
            );
            if malformed {
                let receipt = yolo_hub_agent_protocol::CommandReceipt::accepted(
                    uuid::Uuid::from_u128(ALTERNATE_VALUE),
                    intent.fields().request.fields().target,
                )
                .unwrap();
                respond(&mut control, &request, Reply::AgentControl { receipt }).await;
            }
            drop(control);
            assert_eq!(
                reply(response, &intent).await.fields().outcome,
                ControlOutcome::Unknown {}
            );
            assert_eq!(
                statuses.recv().await,
                Some(Status::ControlLost { holder_id: None })
            );
            let response = enqueue(&send, intent.clone()).await;
            assert_eq!(
                reply(response, &intent).await.fields().outcome,
                ControlOutcome::NotSent {
                    reason: NotSentReason::NoWriter
                }
            );
            input_peer.shutdown().await.unwrap();
            let mut cleanup = accept(&listener).await;
            exact_release(&mut cleanup, grant).await;
            closed(&mut terminal).await;
        };
        let (result, ()) = timeout_at(deadline(), async {
            tokio::join!(
                run_with_commands(
                    options(socket, ControlIntent::Acquire),
                    input,
                    io::sink(),
                    sizes,
                    status,
                    commands,
                    std::future::pending()
                ),
                server
            )
        })
        .await
        .unwrap();
        assert_eq!(result.unwrap(), Outcome::Detached);
    }
}

#[tokio::test]
async fn cancellation_during_dispatch_closes_pending_result_and_only_releases_original_grant() {
    let (_directory, socket, listener) = listener();
    let (input, _input_peer) = io::duplex(PIPE_CAPACITY);
    let (_resize, sizes) = watch::channel(None);
    let (status, mut statuses) = mpsc::channel(STATUS_QUEUE_CAPACITY);
    let (send, commands) = channel();
    let (cancel, cancelled) = oneshot::channel();
    let grant = grant();
    let server = async {
        let (mut control, mut terminal) = acquiring(&listener, grant).await;
        assert_eq!(statuses.recv().await, Some(Status::ControlAcquired));
        let intent = interrupt();
        let response = enqueue(&send, intent.clone()).await;
        let request = receive(&mut control).await;
        assert_eq!(
            request.fields().command,
            Command::AgentControl {
                request: intent.fields().request.clone(),
                grant
            }
        );
        cancel.send(()).unwrap();
        assert!(timeout_at(deadline(), response).await.unwrap().is_err());
        closed(&mut control).await;
        let mut cleanup = accept(&listener).await;
        exact_release(&mut cleanup, grant).await;
        closed(&mut terminal).await;
    };
    let (result, ()) = timeout_at(deadline(), async {
        tokio::join!(
            run_with_commands(
                options(socket, ControlIntent::Acquire),
                input,
                io::sink(),
                sizes,
                status,
                commands,
                async {
                    let _ = cancelled.await;
                }
            ),
            server
        )
    })
    .await
    .unwrap();
    assert_eq!(result.unwrap(), Outcome::Detached);
}
