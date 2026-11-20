use super::{constants::*, fixtures::*, peer::*};
use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
    sync::{mpsc, watch},
    time::timeout_at,
};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{OutputChunk, OutputChunkFields, TerminalEvent, TerminalSize};
use yolo_hub_terminal_relay::{ControlIntent, Options, Outcome, Status, run};

const STATUS_CAPACITY: usize = yolo_hub_terminal_relay::STATUS_QUEUE_CAPACITY;

fn options(socket: std::path::PathBuf, intent: ControlIntent) -> Options {
    Options {
        socket,
        entry: hub(),
        target: target(),
        principal: principal(),
        intent,
        size: Some(TerminalSize::default()),
    }
}

#[tokio::test]
async fn controlled_terminal_output_progresses_during_input_rpc_and_eof_only_releases() {
    let (_directory, socket, listener) = listener();
    let options = options(socket, ControlIntent::Acquire);
    let grant = grant();
    let (input, mut input_peer) = io::duplex(PIPE_CAPACITY);
    let (output, mut output_peer) = io::duplex(PIPE_CAPACITY);
    let (_resize, sizes) = watch::channel(options.size);
    let (status, mut statuses) = mpsc::channel(STATUS_CAPACITY);
    let server = async {
        let mut control = accept(&listener).await;
        let request = receive(&mut control).await;
        assert!(
            matches!(request.fields().command, Command::Lease { request } if request.target == target() && request.principal == principal())
        );
        respond(&mut control, &request, Reply::Lease { grant }).await;
        let request = receive(&mut control).await;
        assert!(
            matches!(request.fields().command, Command::Resize { request } if request.grant == grant)
        );
        respond(&mut control, &request, Reply::Completed {}).await;
        let mut terminal = attach(&listener).await;
        assert_eq!(statuses.recv().await, Some(Status::ControlAcquired));
        let mut snapshot = vec![0; SNAPSHOT.len()];
        output_peer.read_exact(&mut snapshot).await.unwrap();
        assert_eq!(snapshot, SNAPSHOT);
        input_peer.write_all(INPUT).await.unwrap();
        let request = receive(&mut control).await;
        assert!(
            matches!(&request.fields().command, Command::Input { request } if request.fields().data == INPUT && request.fields().grant == grant)
        );
        yolo_hub_terminal_wire::send_event(
            &mut terminal,
            &TerminalEvent::Output {
                chunk: OutputChunk::new(OutputChunkFields {
                    target: target(),
                    sequence: yolo_hub_terminal_protocol::FIRST_OUTPUT_SEQUENCE,
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
        respond(&mut control, &request, Reply::Input { duplicate: false }).await;
        input_peer.shutdown().await.unwrap();
        let release = receive(&mut control).await;
        assert_eq!(release.fields().command, Command::Release { grant });
        respond(&mut control, &release, Reply::Completed {}).await;
        closed(&mut control).await;
        closed(&mut terminal).await;
    };
    let (result, ()) = timeout_at(deadline(), async {
        tokio::join!(
            run(
                options,
                input,
                output,
                sizes,
                status,
                std::future::pending()
            ),
            server,
        )
    })
    .await
    .unwrap();
    assert_eq!(result.unwrap(), Outcome::Detached);
}
