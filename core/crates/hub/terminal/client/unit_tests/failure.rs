use crate::support::*;
use tokio::time::Instant;
use yolo_hub_client::ClientError;
use yolo_hub_jsonrpc_transport::TransportError;
use yolo_hub_terminal_protocol::*;
use yolo_hub_terminal_client::validate_event;

#[tokio::test]
async fn duplicate_foreign_and_gapped_output_poison_without_advancing() {
    for foreign in [true, false] {
        let (mut attachment, mut peer) = connected().await;
        let cursor = attachment.cursor();
        let event = TerminalEvent::Heartbeat {
            cursor: OutputCursor {
                target: if foreign { target() } else { cursor.target },
                after_sequence: FIRST_OUTPUT_SEQUENCE,
            },
        };
        yolo_hub_terminal_wire::send_event(&mut peer, &event.clone(), deadline())
            .await
            .unwrap();
        assert!(matches!(
            attachment.next_event(deadline()).await,
            Err(ClientError::InvalidResponse(_))
        ));
        assert_eq!(attachment.cursor(), cursor);
        assert!(!attachment.is_usable());
    }
}

#[tokio::test]
async fn eof_and_malformed_events_cannot_become_natural_exit() {
    let (mut attachment, peer) = connected().await;
    drop(peer);
    assert!(matches!(
        attachment.next_event(deadline()).await,
        Err(ClientError::Transport(TransportError::TruncatedFrame))
    ));
    let (mut attachment, mut peer) = connected().await;
    peer.send(&false, deadline()).await.unwrap();
    assert!(attachment.next_event(deadline()).await.is_err());
    assert!(!attachment.is_usable());
}

#[tokio::test]
async fn preflight_deadline_preserves_connection_but_cancelled_read_retires_it() {
    let (mut attachment, _peer) = connected().await;
    assert!(matches!(
        attachment.next_event(Instant::now()).await,
        Err(ClientError::Transport(TransportError::Deadline))
    ));
    assert!(attachment.is_usable());
    let cancelled = async {
        let next = attachment.next_event(deadline());
        tokio::pin!(next);
        tokio::select! {
            biased;
            _ = &mut next => panic!(),
            _ = std::future::ready(()) => {},
        }
    };
    cancelled.await;
    assert!(!attachment.is_usable());
}

#[test]
fn even_terminal_failures_must_match_the_subscription_origin() {
    let cursor = OutputCursor {
        target: target(),
        after_sequence: 0,
    };
    for event in [
        TerminalEvent::ResyncRequired { target: target() },
        TerminalEvent::Unreachable { target: target() },
    ] {
        assert_eq!(
            validate_event(&event, cursor),
            Err(PublicError::TargetMismatch)
        );
    }
}
