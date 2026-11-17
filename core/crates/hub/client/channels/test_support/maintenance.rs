use crate::{harness::Harness, peer::*};
use uuid::Uuid;
use yolo_hub_client_channels::{ControlRequest, RequestValidity};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::LeaseGrant;

pub async fn send(
    harness: &Harness,
    generation: Uuid,
    command: Command,
    validity: Option<RequestValidity>,
) {
    harness
        .maintenance
        .as_ref()
        .unwrap()
        .send(ControlRequest {
            validity,
            generation: Some(generation),
            destination: harness.endpoint.hub,
            command,
        })
        .await
        .unwrap();
}

pub async fn renew(peer: &mut Peer, grant: LeaseGrant) {
    let request = receive(peer).await;
    assert_eq!(request.fields().command, Command::Renew { grant });
    reply(peer, &request, Reply::Lease { grant }).await;
}
