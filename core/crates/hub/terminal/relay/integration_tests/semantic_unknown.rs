use super::{agent::*, command_peer::*, constants::*, fixtures::*, peer::*, settings::*};
use tokio::{
    io::{self, AsyncWriteExt},
    sync::{mpsc, watch},
    time::timeout_at,
};
use yolo_hub_agent_protocol::{CommandOutcome, CommandReceipt};
use yolo_hub_relay_control_protocol::Outcome as ControlOutcome;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::PublicError;
use yolo_hub_terminal_relay::{
    ControlIntent, Outcome, STATUS_QUEUE_CAPACITY, Status, run_with_commands,
};

#[tokio::test]
async fn uncertain_origin_result_never_replays_interrupt_or_replaces_the_retained_writer() {
    for explicit_receipt in [false, true] {
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
            let mut receipt = CommandReceipt::accepted(
                intent.fields().request.fields().request_id,
                intent.fields().request.fields().target,
            )
            .unwrap();
            receipt.advance(CommandOutcome::Unknown {}).unwrap();
            let origin_reply = if explicit_receipt {
                Reply::AgentControl { receipt }
            } else {
                Reply::Error {
                    code: PublicError::OutcomeUnknown,
                }
            };
            respond(&mut control, &request, origin_reply).await;
            let expected = if explicit_receipt {
                ControlOutcome::Receipt { receipt }
            } else {
                ControlOutcome::Unknown {}
            };
            assert_eq!(reply(response, &intent).await.fields().outcome, expected);
            assert!(matches!(
                statuses.try_recv(),
                Err(mpsc::error::TryRecvError::Empty)
            ));
            // Closing terminal input is an explicit detach. The only following
            // RPC must release the original grant, not retry the unknown action.
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
