use super::super::{
    federation_agents::ping,
    federation_support::deadline,
    support::{MAX_OBSERVED_BYTES, input},
};
use tokio::io::{AsyncRead, AsyncWrite};
use yolo_hub_client::LocalClient;
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::{LeaseGrant, TerminalEvent};
use yolo_hub_terminal_client::Attachment;

pub async fn confirm(
    client: &mut LocalClient,
    grant: LeaseGrant,
    output: &mut Attachment<impl AsyncRead + AsyncWrite + Unpin>,
    forbidden: Option<&str>,
) {
    let target = grant.fields().target;
    let (command, marker) = ping();
    let reply = client
        .call(target.address.hub_id, input(grant, &command), deadline())
        .await
        .unwrap();
    assert_eq!(reply, Reply::Input { duplicate: false });
    let mut observed = output
        .initial()
        .fields()
        .snapshot
        .as_ref()
        .unwrap()
        .fields()
        .data
        .clone();
    for chunk in &output.initial().fields().chunks {
        observed.extend_from_slice(&chunk.fields().data);
    }
    assert!(observed.len() <= MAX_OBSERVED_BYTES);
    tokio::time::timeout_at(deadline(), async {
        loop {
            let text = String::from_utf8_lossy(&observed);
            if let Some(forbidden) = forbidden {
                assert!(!text.contains(forbidden));
            }
            if text.contains(&marker) {
                break;
            }
            let event = output.next_event(deadline()).await.unwrap();
            assert_eq!(event.target(), target);
            assert!(!event.is_terminal());
            if let TerminalEvent::Output { chunk } = event {
                observed.extend_from_slice(&chunk.fields().data);
                assert!(observed.len() <= MAX_OBSERVED_BYTES);
            }
        }
    })
    .await
    .unwrap();
}
