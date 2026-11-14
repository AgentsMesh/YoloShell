use super::{constants::*, fixtures::*, peer::*, settings::*};
use tokio::{
    io,
    sync::{mpsc, oneshot, watch},
    time::timeout_at,
};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_relay::{ControlIntent, Outcome, STATUS_QUEUE_CAPACITY, Status, run};

#[tokio::test]
async fn cancellation_retires_an_incomplete_initial_handshake_without_waiting_its_deadline() {
    let (_directory, socket, listener) = listener();
    let (cancel, cancelled) = oneshot::channel();
    let (_resize, sizes) = watch::channel(None);
    let (status, _statuses) = mpsc::channel(STATUS_QUEUE_CAPACITY);
    let server = async {
        let (stream, _) = listener.accept().await.unwrap();
        let mut peer = yolo_hub_jsonrpc_transport::Framed::new(stream);
        let request = receive(&mut peer).await;
        assert_eq!(request.fields().command, Command::Hello {});
        cancel.send(()).unwrap();
        closed(&mut peer).await;
    };
    let (result, ()) = timeout_at(deadline(), async {
        tokio::join!(
            run(
                options(socket, ControlIntent::Acquire),
                io::empty(),
                io::sink(),
                sizes,
                status,
                async {
                    let _ = cancelled.await;
                }
            ),
            server,
        )
    })
    .await
    .unwrap();
    assert_eq!(result.unwrap(), Outcome::Detached);
}

#[tokio::test]
async fn blocked_output_does_not_starve_renewal_and_parent_cancellation_releases_known_grant() {
    let (_directory, socket, listener) = listener();
    let (input, _input_peer) = io::duplex(PIPE_CAPACITY);
    let (output, _output_peer) = io::duplex(SNAPSHOT.len());
    let (cancel, cancelled) = oneshot::channel();
    let (_resize, sizes) = watch::channel(None);
    let (status, mut statuses) = mpsc::channel(STATUS_QUEUE_CAPACITY);
    let grant = grant();
    let server = async {
        let (mut control, mut terminal) = acquiring(&listener, grant).await;
        assert_eq!(statuses.recv().await, Some(Status::ControlAcquired));
        use yolo_hub_terminal_protocol::{
            FIRST_OUTPUT_SEQUENCE, OutputChunk, OutputChunkFields, TerminalEvent,
        };
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
        let renewal = receive(&mut control).await;
        assert_eq!(renewal.fields().command, Command::Renew { grant });
        respond(&mut control, &renewal, Reply::Lease { grant }).await;
        cancel.send(()).unwrap();
        // Cancellation may retire a partially-read control reply. Release is always
        // exact-capability, whether the original connection remains usable or not.
        let incoming = match control
            .receive::<yolo_hub_jsonrpc_protocol::Call>(deadline())
            .await
        {
            Ok(incoming) => incoming,
            // Linux may reset a socket closed with an unread renewal reply.
            // This still requires exact-capability release on the fresh channel.
            Err(yolo_hub_jsonrpc_transport::TransportError::Io(error))
                if error.kind() == std::io::ErrorKind::ConnectionReset =>
            {
                yolo_hub_jsonrpc_transport::Incoming::Eof
            }
            Err(error) => panic!("{error}"),
        };
        match incoming {
            yolo_hub_jsonrpc_transport::Incoming::Message(release) => {
                assert_eq!(release.fields().command, Command::Release { grant });
                respond(&mut control, &release, Reply::Completed {}).await;
            }
            yolo_hub_jsonrpc_transport::Incoming::Eof => {
                let mut reconnect = accept(&listener).await;
                let release = receive(&mut reconnect).await;
                assert_eq!(release.fields().command, Command::Release { grant });
                respond(&mut reconnect, &release, Reply::Completed {}).await;
                closed(&mut reconnect).await;
            }
        }
        closed(&mut terminal).await;
    };
    let (result, ()) = timeout_at(deadline(), async {
        tokio::join!(
            run(
                options(socket, ControlIntent::Acquire),
                input,
                output,
                sizes,
                status,
                async {
                    let _ = cancelled.await;
                }
            ),
            server,
        )
    })
    .await
    .unwrap();
    assert_eq!(result.unwrap(), Outcome::Detached);
}
