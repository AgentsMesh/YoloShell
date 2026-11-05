use super::{
    fixture::Fixture, listener_initialization_fixture::InitialStartup, listener_restart::reopen,
};
use crate::*;
use std::num::NonZeroU16;
use tokio::sync::watch;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_dispatch::{Dispatcher, Operation};
use yolo_hub_enrollment_protocol::{
    InviteDial, InvitePolicy, InviteRequest, MintedInvite, SignedInvite, decode,
};
use yolo_hub_enrollment_wire::{JOIN_SCHEMA_VERSION, JoinErrorCode, JoinRequest};
use yolo_hub_peer_transport::JoinFailure;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, Reply, Request};

const INVITE_LIFETIME_MS: u64 = 10 * 60 * 1000;
const JOIN_DEADLINE: std::time::Duration = std::time::Duration::from_secs(15);

fn address_dial(address: std::net::SocketAddr) -> InviteDial {
    InviteDial {
        host: address.ip().to_string(),
        port: NonZeroU16::new(address.port()).unwrap(),
    }
}

async fn mint(hub: &ManagedHub, dial: Option<InviteDial>) -> (String, SignedInvite) {
    let request = Request::new(
        hub.hub_id(),
        Command::MintInvite {
            request: InviteRequest {
                dial,
                lifetime_ms: INVITE_LIFETIME_MS,
                policy: InvitePolicy {
                    export_local: true,
                    import_peer: false,
                },
            },
        },
    )
    .unwrap();
    let Operation::Reply(Reply::Minted { invite }) = hub.dispatch(request).await.unwrap() else {
        panic!("expected a minted invite")
    };
    let MintedInvite { code, .. } = invite;
    let signed = decode(&code).unwrap();
    (code, signed)
}

fn join_request(invite: &SignedInvite, peer: PeerIdentity) -> JoinRequest {
    JoinRequest {
        schema_version: JOIN_SCHEMA_VERSION,
        request_id: uuid::Uuid::new_v4(),
        invite_id: invite.claims().invite_id,
        peer,
    }
}

async fn join(
    address: std::net::SocketAddr,
    code: &str,
    invite: &SignedInvite,
    request: &JoinRequest,
) -> Result<yolo_hub_enrollment_wire::JoinReply, JoinFailure> {
    yolo_hub_peer_transport::join(
        &address.ip().to_string(),
        NonZeroU16::new(address.port()).unwrap(),
        code,
        invite,
        request,
        tokio::time::Instant::now() + JOIN_DEADLINE,
    )
    .await
}

async fn snapshot(hub: &ManagedHub) -> ConfigurationSnapshot {
    let request = Request::new(hub.hub_id(), Command::GetConfiguration {}).unwrap();
    let Operation::Reply(Reply::Configuration { snapshot }) = hub.dispatch(request).await.unwrap()
    else {
        panic!("expected configuration")
    };
    snapshot
}

#[tokio::test]
async fn enrollment_adds_the_joining_hub_once_and_rejects_replay() {
    let startup = InitialStartup::new();
    let (directory, backend, _) = startup.release();
    let Fixture {
        directory: _directory,
        hub,
        service,
        state: _state,
    } = reopen(directory, backend).await.unwrap();
    let address = service.listen_address().unwrap().unwrap();
    let (code, invite) = mint(&hub, Some(address_dial(address))).await;
    let (shutdown, stopped) = watch::channel(false);
    let task = tokio::spawn(service.run(stopped));
    let peer = PeerIdentity::new(HubId::new(), [0x42; 32]).unwrap();
    let reply = join(address, &code, &invite, &join_request(&invite, peer))
        .await
        .unwrap();
    assert_eq!(reply.hub, hub.state.peer_identity());
    assert!(reply.applied);
    let applied = snapshot(&hub).await;
    let configured = applied.state.configuration.peer(peer.hub()).unwrap();
    assert_eq!(configured.identity, peer);
    assert_eq!(configured.exports.len(), 1);

    let retried = join(address, &code, &invite, &join_request(&invite, peer))
        .await
        .unwrap();
    assert!(retried.applied);

    let other = PeerIdentity::new(HubId::new(), [0x43; 32]).unwrap();
    assert_eq!(
        join(address, &code, &invite, &join_request(&invite, other))
            .await
            .unwrap_err(),
        JoinFailure::Rejected(JoinErrorCode::Unauthorized)
    );
    assert!(applied.state.configuration.peer(other.hub()).is_none());

    shutdown.send_replace(true);
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn an_address_free_invite_joins_with_an_explicit_address() {
    let startup = InitialStartup::new();
    let (directory, backend, _) = startup.release();
    let Fixture {
        directory: _directory,
        hub,
        service,
        state: _state,
    } = reopen(directory, backend).await.unwrap();
    let address = service.listen_address().unwrap().unwrap();
    let (code, invite) = mint(&hub, None).await;
    assert!(code.starts_with("yolo2_"), "{code}");
    let (shutdown, stopped) = watch::channel(false);
    let task = tokio::spawn(service.run(stopped));
    let peer = PeerIdentity::new(HubId::new(), [0x44; 32]).unwrap();
    let reply = join(address, &code, &invite, &join_request(&invite, peer))
        .await
        .unwrap();
    assert_eq!(reply.hub, hub.state.peer_identity());
    assert!(reply.applied);
    shutdown.send_replace(true);
    task.await.unwrap().unwrap();
}
