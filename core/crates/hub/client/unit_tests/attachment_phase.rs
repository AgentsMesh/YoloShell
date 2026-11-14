use super::{fixtures::*, server::*};
use crate::ClientError;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::*;

#[tokio::test]
async fn attachment_transitions_to_a_read_only_event_channel() {
    let (mut client, mut peer) = connected().await;
    let attach = AttachRequest::new(target(), None).unwrap();
    let initial = AttachResult::new(AttachResultFields {
        target: target(),
        snapshot: Some(
            TerminalSnapshot::new(TerminalSnapshotFields {
                target: target(),
                barrier_sequence: 0,
                size: TerminalSize::default(),
                data: Vec::new(),
            })
            .unwrap(),
        ),
        chunks: Vec::new(),
        history: None,
        next_sequence: FIRST_OUTPUT_SEQUENCE,
        resynchronized: true,
    })
    .unwrap();
    let response = async {
        let request = receive(&mut peer).await;
        respond(
            &mut peer,
            &request,
            Reply::Attached {
                attachment: initial,
            },
        )
        .await;
    };
    let (result, ()) = tokio::join!(
        client.call(hub(), Command::Attach { request: attach }, deadline()),
        response
    );
    result.unwrap();
    assert!(!client.is_usable());
    assert!(matches!(
        client.call(hub(), input(), deadline()).await,
        Err(ClientError::TerminalStream)
    ));
    let cursor = OutputCursor {
        target: target(),
        after_sequence: 0,
    };
    yolo_hub_terminal_wire::send_event(&mut peer, &TerminalEvent::Heartbeat { cursor }, deadline())
        .await
        .unwrap();
    let mut events = client.into_channel().unwrap();
    assert!(matches!(
        yolo_hub_terminal_wire::receive_event(&mut events, deadline())
            .await
            .unwrap(),
        yolo_hub_jsonrpc_transport::Incoming::Message(TerminalEvent::Heartbeat { .. })
    ));
}
