use super::{agent::*, command_peer::*, constants::*, fixtures::*, peer::*, settings::*};
use tokio::{
    io::{self, AsyncWriteExt},
    sync::{mpsc, watch},
    time::timeout_at,
};
use yolo_hub_agent_protocol::{CommandOutcome, FailureCode};
use yolo_hub_relay_control_protocol::{NotSentReason, Outcome as ControlOutcome};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::PublicError;
use yolo_hub_terminal_relay::{
    ControlIntent, Outcome, STATUS_QUEUE_CAPACITY, Status, run_with_commands,
};

#[tokio::test]
async fn origin_rejections_preserve_semantics_and_retire_stale_authority_without_retry() {
    for (remote, failure, loses_writer) in [
        (PublicError::Unsupported, FailureCode::Unsupported, false),
        (
            PublicError::Conflict,
            FailureCode::PreconditionsChanged,
            true,
        ),
        (
            PublicError::TargetMismatch,
            FailureCode::PreconditionsChanged,
            true,
        ),
        (
            PublicError::Unauthorized,
            FailureCode::PermissionDenied,
            true,
        ),
    ] {
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
            respond(&mut control, &request, Reply::Error { code: remote }).await;
            let response = reply(response, &intent).await;
            let ControlOutcome::Receipt { receipt } = response.fields().outcome else {
                panic!()
            };
            assert_eq!(receipt.outcome(), CommandOutcome::Failed { code: failure });
            if loses_writer {
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
            } else {
                assert!(matches!(
                    statuses.try_recv(),
                    Err(mpsc::error::TryRecvError::Empty)
                ));
            }
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
}
