use super::deadline;
use yolo_hub_client::{ClientError, HubEntryStream};
use yolo_hub_terminal_protocol::TerminalEvent;
use yolo_hub_terminal_client::Attachment;
use yolo_hub_jsonrpc_transport::TransportError;

pub async fn until<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(
    output: &mut Attachment<S>,
    marker: &str,
) -> Vec<u8> {
    let mut bytes = output
        .initial()
        .fields()
        .snapshot
        .as_ref()
        .map(|snapshot| snapshot.fields().data.clone())
        .unwrap_or_default();
    tokio::time::timeout_at(deadline(), async {
        while !String::from_utf8_lossy(&bytes).contains(marker) {
            match output.next_event(deadline()).await.unwrap() {
                TerminalEvent::Output { chunk } => bytes.extend_from_slice(&chunk.fields().data),
                event if event.is_terminal() => panic!("terminal closed before marker"),
                _ => {}
            }
            assert!(bytes.len() <= crate::hub::support::MAX_OBSERVED_BYTES);
        }
    })
    .await
    .unwrap();
    bytes
}

pub fn assert_closed(error: ClientError) {
    match error {
        ClientError::Poisoned
        | ClientError::Transport(TransportError::TruncatedFrame)
        | ClientError::Transport(TransportError::Poisoned) => {}
        ClientError::Transport(TransportError::Io(error))
        | ClientError::OutcomeUnknown(TransportError::Io(error)) => assert!(
            matches!(
                error.kind(),
                std::io::ErrorKind::UnexpectedEof
                    | std::io::ErrorKind::BrokenPipe
                    | std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::NotConnected
                    | std::io::ErrorKind::ConnectionAborted
            ),
            "unexpected disconnect IO: {error}"
        ),
        ClientError::OutcomeUnknown(TransportError::TruncatedFrame) => {}
        other => panic!("channel did not prove closure: {other}"),
    }
}

pub async fn closed(output: &mut Attachment<HubEntryStream>) {
    tokio::time::timeout_at(deadline(), async {
        loop {
            match output.next_event(deadline()).await {
                Err(error) => {
                    assert_closed(error);
                    return;
                }
                Ok(TerminalEvent::Output { .. } | TerminalEvent::Heartbeat { .. }) => {}
                Ok(TerminalEvent::Unreachable { .. }) => {}
                Ok(_) => panic!("revocation cannot exit or replace the origin runtime"),
            }
        }
    })
    .await
    .unwrap();
    assert!(!output.is_usable());
}
