use super::{agent::*, command_peer::*, constants::*, fixtures::*, peer::*, settings::*};
use tokio::{
    io::{self, AsyncWriteExt},
    sync::{mpsc, watch},
    time::timeout_at,
};
use uuid::Uuid;
use yolo_hub_identity::RuntimeEpoch;
use yolo_hub_relay_control_protocol::{NotSentReason, Outcome as ControlOutcome, Request};
use yolo_hub_terminal_relay::{
    ControlIntent, Outcome, STATUS_QUEUE_CAPACITY, Status, run_with_commands,
};

#[tokio::test]
async fn wrong_launch_runtime_and_cancelled_queued_intents_never_dispatch_or_acquire() {
    let (_directory, socket, listener) = listener();
    let (input, mut input_peer) = io::duplex(PIPE_CAPACITY);
    let (_resize, sizes) = watch::channel(None);
    let (status, mut statuses) = mpsc::channel(STATUS_QUEUE_CAPACITY);
    let (send, commands) = channel();
    let grant = grant();
    let server = async {
        let (mut control, mut terminal) = acquiring(&listener, grant).await;
        assert_eq!(statuses.recv().await, Some(Status::ControlAcquired));
        let mut wrong_launch = interrupt().fields().clone();
        wrong_launch.launch_id = Uuid::from_u128(ALTERNATE_VALUE);
        let mut wrong_runtime = interrupt().fields().clone();
        wrong_runtime.terminal.runtime_epoch =
            RuntimeEpoch::from_uuid(Uuid::from_u128(ALTERNATE_VALUE)).unwrap();
        for fields in [wrong_launch, wrong_runtime] {
            let intent = Request::new(fields).unwrap();
            let response = enqueue(&send, intent.clone()).await;
            assert_eq!(
                reply(response, &intent).await.fields().outcome,
                ControlOutcome::NotSent {
                    reason: NotSentReason::TargetChanged
                }
            );
        }
        let (response, abandoned) = tokio::sync::oneshot::channel();
        drop(abandoned);
        send.send(yolo_hub_terminal_relay::ControlCommand {
            request: interrupt(),
            response,
        })
        .await
        .unwrap();
        input_peer.shutdown().await.unwrap();
        exact_release(&mut control, grant).await;
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

#[tokio::test]
async fn readonly_interrupt_returns_no_writer_without_sending_control_or_lease() {
    let (_directory, socket, listener) = listener();
    let (input, mut input_peer) = io::duplex(PIPE_CAPACITY);
    let (_resize, sizes) = watch::channel(None);
    let (status, mut statuses) = mpsc::channel(STATUS_QUEUE_CAPACITY);
    let (send, commands) = channel();
    let server = async {
        let (mut control, mut terminal) = observing(&listener).await;
        assert_eq!(
            statuses.recv().await,
            Some(Status::ReadOnly { holder_id: None })
        );
        let intent = interrupt();
        let response = enqueue(&send, intent.clone()).await;
        assert_eq!(
            reply(response, &intent).await.fields().outcome,
            ControlOutcome::NotSent {
                reason: NotSentReason::NoWriter
            }
        );
        input_peer.shutdown().await.unwrap();
        closed(&mut control).await;
        closed(&mut terminal).await;
    };
    let (result, ()) = timeout_at(deadline(), async {
        tokio::join!(
            run_with_commands(
                options(socket, ControlIntent::Observe),
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
