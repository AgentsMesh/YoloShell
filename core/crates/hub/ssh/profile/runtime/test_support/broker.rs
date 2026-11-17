use super::constants::*;
use std::os::unix::fs::PermissionsExt;
use tokio::net::{UnixListener, UnixStream};
use yolo_broker_hub_protocol::{BrokerHubConnectionBinding, BrokerHubOpened, BrokerHubProfile};
use yolo_hub_delegation::PeerIdentity;
use yolo_remote_broker::{
    BROKER_PROTOCOL_VERSION, BrokerPaths, BrokerRequest, BrokerRequestEnvelope, BrokerResponse,
    BrokerResult,
};

pub fn bind(root: &std::path::Path) -> (BrokerPaths, UnixListener) {
    let paths = BrokerPaths::new(root).unwrap();
    let listener = UnixListener::bind(paths.management_socket()).unwrap();
    std::fs::set_permissions(
        paths.management_socket(),
        std::fs::Permissions::from_mode(SOCKET_MODE),
    )
    .unwrap();
    (paths, listener)
}
pub async fn upgrade(listener: &UnixListener, peer: PeerIdentity) -> UnixStream {
    let (mut preparation, _) = listener.accept().await.unwrap();
    let envelope: BrokerRequestEnvelope = yolo_wire::read_frame(&mut preparation).await.unwrap();
    let BrokerRequest::PrepareHubTransport(profile) = envelope.request else {
        panic!()
    };
    let binding = binding(profile);
    reply(
        &mut preparation,
        envelope.request_id,
        BrokerResult::HubTransport(binding.clone()),
    )
    .await;
    let (mut tunnel, _) = listener.accept().await.unwrap();
    let envelope: BrokerRequestEnvelope = yolo_wire::read_frame(&mut tunnel).await.unwrap();
    let BrokerRequest::OpenHubPeer(request) = envelope.request else {
        panic!()
    };
    assert_eq!(request.binding, binding);
    assert_eq!(request.peer, peer);
    assert_eq!(request.endpoint.to_string(), ENDPOINT);
    reply(
        &mut tunnel,
        envelope.request_id,
        BrokerResult::HubPeer(BrokerHubOpened {
            request,
            channel_id: CHANNEL,
        }),
    )
    .await;
    tunnel
}
pub fn binding(profile: BrokerHubProfile) -> BrokerHubConnectionBinding {
    BrokerHubConnectionBinding {
        profile,
        broker_instance_id: uuid::Uuid::new_v4(),
        connection_id: uuid::Uuid::new_v4(),
        connection_generation: GENERATION,
    }
}
pub async fn reply(stream: &mut UnixStream, request_id: uuid::Uuid, result: BrokerResult) {
    yolo_wire::write_frame(
        stream,
        &BrokerResponse {
            protocol_version: BROKER_PROTOCOL_VERSION,
            request_id,
            result: Some(result),
            error_code: None,
            error_message: None,
        },
    )
    .await
    .unwrap();
}
