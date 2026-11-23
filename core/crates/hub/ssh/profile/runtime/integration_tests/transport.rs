use super::{broker, constants::*, fixture::*};
use std::sync::Arc;
use tokio::time::timeout_at;
use yolo_hub_federation_settings::PeerAddress;
use yolo_hub_peer_dialer::dial_peer;
use yolo_hub_peer_link::{Admission, accept, connect};
use yolo_hub_peer_protocol::{FIRST_REVISION, PEER_PROTOCOL_VERSION, PeerReply, PeerRequest};
use yolo_hub_peer_rpc::{Client, Handler, Response, RpcFuture};
use yolo_hub_state::HubState;
use uuid::Uuid;

struct Echo;
impl Handler for Echo {
    fn request(&self, request: PeerRequest) -> RpcFuture<'_, Response> {
        Box::pin(async move {
            let PeerRequest::Discover { request_id, .. } = request else {
                return Err(yolo_hub_peer_rpc::RpcError::Invalid);
            };
            Ok(Response {
                reply: PeerReply::Directory {
                    request_id,
                    revision: FIRST_REVISION,
                    origins: Vec::new(),
                },
                updates: None,
            })
        })
    }
}

fn discover() -> PeerRequest {
    PeerRequest::Discover {
        request_id: Uuid::new_v4(),
        version: PEER_PROTOCOL_VERSION,
        allowed_origins: Vec::new(),
    }
}

#[tokio::test]
async fn unknown_profile_never_opens_a_remote_socket() {
    let fixture = Fixture::new();
    let root = tempfile::tempdir().unwrap();
    let socket = root.path().join("hub.sock");
    let listener = tokio::net::UnixListener::bind(&socket).unwrap();
    let address = PeerAddress::profile(PROFILE_ID.to_owned(), socket).unwrap();
    assert!(
        dial_peer(
            fixture.runtime.as_ref(),
            fixture.state.peer_identity(),
            &address,
            deadline()
        )
        .await
        .is_err()
    );
    let incoming =
        tokio::select! { _ = listener.accept() => true, _ = tokio::task::yield_now() => false };
    assert!(!incoming);
}

#[tokio::test]
async fn registry_removal_closes_the_existing_link_before_the_change_reply() {
    let fixture = Fixture::new();
    let (paths, listener) = broker::bind(&fixture.root.path().join(BROKER_DIRECTORY));
    let profile = fixture.profile(paths.state_dir());
    fixture.apply(vec![profile.clone()]).await;
    let origin = Arc::new(
        HubState::open(
            &fixture.root.path().join(ORIGIN_DIRECTORY),
            &fixture.root.path().join(ORIGIN_HOST_DIRECTORY),
        )
        .unwrap(),
    );
    let peer = origin.peer_identity();
    let address = PeerAddress::profile(profile.profile.profile_id().to_owned(), SOCKET.into()).unwrap();
    let (transport, remote) = tokio::join!(
        dial_peer(fixture.runtime.as_ref(), peer, &address, deadline()),
        timeout_at(deadline(), broker::upgrade(&listener, peer)),
    );
    let local = fixture.state.peer_identity();
    let stream = remote.unwrap();
    let server_origin = origin.clone();
    let served = tokio::spawn(async move {
        let (_, stream) = accept(stream, peer, server_origin.as_ref(), Admission::Only(&[local]), deadline())
            .await
            .unwrap();
        let _ = yolo_hub_peer_rpc::serve(stream, Arc::new(Echo)).await;
    });
    let client: Client = connect(
        transport.unwrap(),
        local,
        fixture.state.as_ref(),
        peer,
        deadline(),
    )
    .await
    .unwrap();
    assert!(client.request(discover(), deadline()).await.is_ok());
    fixture.apply(Vec::new()).await;
    timeout_at(deadline(), async {
        while client.request(discover(), deadline()).await.is_ok() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    timeout_at(deadline(), served).await.unwrap().unwrap();
    paths.verify_state().unwrap();
}
