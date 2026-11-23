use super::{agent::*, command_peer::*, constants::*, fixtures::*, peer::*, settings::*};
use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
    sync::{mpsc, watch},
    time::timeout_at,
};
use yolo_hub_relay_control_protocol::Outcome as ControlOutcome;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{
    FIRST_OUTPUT_SEQUENCE, OutputChunk, OutputChunkFields, TerminalEvent,
};
use yolo_hub_terminal_relay::{
    ControlIntent, Outcome, STATUS_QUEUE_CAPACITY, Status, run_with_commands,
};

#[tokio::test]
async fn semantic_interrupt_uses_retained_grant_and_never_blocks_terminal_output() {
    let (_directory, socket, listener) = listener();
    let (input, mut input_peer) = io::duplex(PIPE_CAPACITY);
    let (output, mut output_peer) = io::duplex(PIPE_CAPACITY);
    let (_resize, sizes) = watch::channel(None);
    let (status, mut statuses) = mpsc::channel(STATUS_QUEUE_CAPACITY);
    let (send, commands) = channel();
    let grant = grant();
    let server = async {
        let (mut control, mut terminal) = acquiring(&listener, grant).await;
        assert_eq!(statuses.recv().await, Some(Status::ControlAcquired));
        let mut snapshot = vec![0; SNAPSHOT.len()];
        output_peer.read_exact(&mut snapshot).await.unwrap();
        assert_eq!(snapshot, SNAPSHOT);
        let intent = interrupt();
        let response = enqueue(&send, intent.clone()).await;
        let request = receive(&mut control).await;
        assert_eq!(request.fields().destination, origin());
        assert_eq!(
            request.fields().command,
            Command::AgentControl {
                request: intent.fields().request.clone(),
                grant
            }
        );
        yolo_hub_terminal_wire::send_event(&mut terminal, &TerminalEvent::Output {
                    chunk: OutputChunk::new(OutputChunkFields {
                        target: target(),
                        sequence: FIRST_OUTPUT_SEQUENCE,
                        data: OUTPUT.to_vec(),
                    })
                    .unwrap(),
                },
                deadline(),
            )
            .await
            .unwrap();
        let mut bytes = vec![0; OUTPUT.len()];
        output_peer.read_exact(&mut bytes).await.unwrap();
        assert_eq!(bytes, OUTPUT);
        let receipt = delivered(&intent);
        respond(&mut control, &request, Reply::AgentControl { receipt }).await;
        assert_eq!(
            reply(response, &intent).await.fields().outcome,
            ControlOutcome::Receipt { receipt }
        );
        input_peer.shutdown().await.unwrap();
        exact_release(&mut control, grant).await;
        closed(&mut terminal).await;
    };
    let (result, ()) = timeout_at(deadline(), async {
        tokio::join!(
            run_with_commands(
                options(socket, ControlIntent::Acquire),
                input,
                output,
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
