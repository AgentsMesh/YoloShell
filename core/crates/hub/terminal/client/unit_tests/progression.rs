use crate::support::*;
use yolo_hub_client::ClientError;
use yolo_hub_jsonrpc_transport::Incoming;
use yolo_hub_terminal_protocol::*;

#[tokio::test]
async fn snapshot_live_output_heartbeat_and_exit_keep_exact_progress() {
    let (mut attachment, mut peer) = connected().await;
    let initial = attachment.initial();
    assert!(initial.fields().resynchronized);
    assert_eq!(attachment.cursor().after_sequence, 0);
    assert!(attachment.is_usable());
    let event = output(attachment.cursor());
    yolo_hub_terminal_wire::send_event(&mut peer, &event.clone(), deadline())
        .await
        .unwrap();
    assert_eq!(attachment.next_event(deadline()).await.unwrap(), event);
    assert_eq!(attachment.cursor().after_sequence, FIRST_OUTPUT_SEQUENCE);
    let heartbeat = TerminalEvent::Heartbeat {
        cursor: attachment.cursor(),
    };
    yolo_hub_terminal_wire::send_event(&mut peer, &heartbeat.clone(), deadline())
        .await
        .unwrap();
    assert_eq!(attachment.next_event(deadline()).await.unwrap(), heartbeat);
    let exited = TerminalEvent::Exited {
        cursor: attachment.cursor(),
        code: Some(0),
        signal: None,
    };
    yolo_hub_terminal_wire::send_event(&mut peer, &exited.clone(), deadline())
        .await
        .unwrap();
    assert_eq!(attachment.next_event(deadline()).await.unwrap(), exited);
    assert!(!attachment.is_usable());
    assert!(matches!(
        attachment.next_event(deadline()).await,
        Err(ClientError::Poisoned)
    ));
    assert!(matches!(
        attachment.detach(deadline()).await,
        Err(ClientError::Poisoned)
    ));
}

#[tokio::test]
async fn explicit_gap_and_unreachable_are_terminal_events_not_process_exits() {
    for gap in [true, false] {
        let (mut attachment, mut peer) = connected().await;
        let cursor = attachment.cursor();
        let event = if gap {
            TerminalEvent::ResyncRequired {
                target: cursor.target,
            }
        } else {
            TerminalEvent::Unreachable {
                target: cursor.target,
            }
        };
        yolo_hub_terminal_wire::send_event(&mut peer, &event.clone(), deadline())
            .await
            .unwrap();
        assert_eq!(attachment.next_event(deadline()).await.unwrap(), event);
        assert_eq!(attachment.cursor(), cursor);
        assert!(!attachment.is_usable());
    }
}

#[tokio::test]
async fn detach_only_closes_the_subscription() {
    let (attachment, mut peer) = connected().await;
    attachment.detach(deadline()).await.unwrap();
    assert_eq!(
        peer.receive::<yolo_hub_jsonrpc_protocol::Call>(deadline())
            .await
            .unwrap(),
        Incoming::Eof
    );
}
