use super::super::{
    federation_agents::ping,
    federation_support::{RunningNode, deadline},
    support::{MAX_OBSERVED_BYTES, acquire, input},
};
use tokio::io::{AsyncRead, AsyncWrite};
use yolo_hub_client::ClientError;
use yolo_hub_identity::RuntimeTarget;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{AttachRequest, LeaseMode, OutputCursor, TerminalEvent};
use yolo_hub_terminal_client::Attachment;
use yolo_hub_jsonrpc_transport::TransportError;

pub async fn attach(
    node: &RunningNode,
    target: RuntimeTarget,
    cursor: Option<OutputCursor>,
) -> Attachment<impl AsyncRead + AsyncWrite + Unpin> {
    attach_client(node.client().await, target, cursor).await
}

pub async fn attach_client(
    client: yolo_hub_client::LocalClient,
    target: RuntimeTarget,
    cursor: Option<OutputCursor>,
) -> Attachment<impl AsyncRead + AsyncWrite + Unpin> {
    let output = Attachment::open(
        client,
        AttachRequest::new(target, cursor).unwrap(),
        deadline(),
    )
    .await
    .unwrap();
    assert_eq!(output.cursor().target, target);
    if let Some(cursor) = cursor {
        assert!(output.cursor().after_sequence >= cursor.after_sequence);
    }
    output
}

pub async fn exchange(
    node: &RunningNode,
    target: RuntimeTarget,
    output: &mut Attachment<impl AsyncRead + AsyncWrite + Unpin>,
) -> OutputCursor {
    exchange_client(node.client().await, target, output).await
}

pub async fn exchange_client(
    client: yolo_hub_client::LocalClient,
    target: RuntimeTarget,
    output: &mut Attachment<impl AsyncRead + AsyncWrite + Unpin>,
) -> OutputCursor {
    let (command, marker) = ping();
    exchange_command_client(client, target, output, &command, &marker).await
}

pub async fn exchange_command_client(
    mut client: yolo_hub_client::LocalClient,
    target: RuntimeTarget,
    output: &mut Attachment<impl AsyncRead + AsyncWrite + Unpin>,
    command: &[u8],
    marker: &str,
) -> OutputCursor {
    let Reply::Lease { grant } = client
        .call(
            target.address.hub_id,
            acquire(target, LeaseMode::Acquire {}),
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    let Reply::Input { duplicate } = client
        .call(target.address.hub_id, input(grant, command), deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(!duplicate);
    let mut received = Vec::new();
    tokio::time::timeout_at(deadline(), async {
        while !String::from_utf8_lossy(&received).contains(marker) {
            let event = output.next_event(deadline()).await.unwrap();
            assert_eq!(event.target(), target);
            assert!(!event.is_terminal());
            if let TerminalEvent::Output { chunk } = event {
                received.extend_from_slice(&chunk.fields().data);
                assert!(received.len() <= MAX_OBSERVED_BYTES);
            }
        }
    })
    .await
    .unwrap();
    client
        .call(
            target.address.hub_id,
            Command::Release { grant },
            deadline(),
        )
        .await
        .unwrap();
    output.cursor()
}

pub async fn expect_transport_loss(
    output: &mut Attachment<impl AsyncRead + AsyncWrite + Unpin>,
    target: RuntimeTarget,
) {
    tokio::time::timeout_at(deadline(), async {
        loop {
            match output.next_event(deadline()).await {
                Err(error) => {
                    assert!(
                        matches!(
                            error,
                            ClientError::Transport(
                                TransportError::TruncatedFrame | TransportError::Io(_)
                            )
                        ),
                        "unexpected direct-route failure: {error}"
                    );
                    break;
                }
                Ok(TerminalEvent::Unreachable { target: observed }) => {
                    assert_eq!(observed, target);
                    break;
                }
                Ok(event) => {
                    assert_eq!(event.target(), target);
                    assert!(
                        !event.is_terminal(),
                        "route loss must not manufacture a process exit"
                    );
                }
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(output.cursor().target, target);
    assert!(!output.is_usable());
}
