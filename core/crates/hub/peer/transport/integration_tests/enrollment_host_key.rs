use crate::enrollment::{Verifier, hub, invite};
use crate::support::*;
use std::{num::NonZeroU16, sync::Arc};
use tokio::net::TcpListener;
use yolo_hub_enrollment_protocol::encode;
use yolo_hub_enrollment_wire::{JOIN_SCHEMA_VERSION, JoinRequest};

#[tokio::test]
async fn enrollment_reports_a_host_key_mismatch() {
    let (_server_signer, transport) = hub();
    let (invite_signer, _) = hub();
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let invite = invite(&invite_signer, NonZeroU16::new(port).unwrap());
    let verifier = Arc::new(Verifier::new(invite.clone()));
    let request = JoinRequest {
        schema_version: JOIN_SCHEMA_VERSION,
        request_id: uuid::Uuid::new_v4(),
        invite_id: invite.claims().invite_id,
        peer: keys().0,
    };
    let served = async {
        let (server, _) = listener.accept().await.unwrap();
        yolo_hub_peer_transport::accept_with_enrollment(
            server,
            transport,
            yolo_hub_peer_transport::PeerPins::new(vec![]).unwrap(),
            Some(verifier),
            deadline(),
        )
        .await
    };
    let code = encode(&invite).unwrap();
    let joined = yolo_hub_peer_transport::join(
        "127.0.0.1",
        NonZeroU16::new(port).unwrap(),
        &code,
        &invite,
        &request,
        deadline(),
    );
    let (accepted, result) = tokio::join!(served, joined);
    assert!(accepted.is_err());
    assert_eq!(
        result.unwrap_err(),
        yolo_hub_peer_transport::JoinFailure::HostKeyMismatch
    );
}
