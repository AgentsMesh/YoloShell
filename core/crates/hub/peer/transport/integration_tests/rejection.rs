use crate::support::*;
use yolo_hub_peer_transport::{PeerPins, accept, connect};

#[tokio::test]
async fn both_server_pin_and_client_identity_are_required() {
    for wrong_server in [true, false] {
        let (client_id, client_key) = keys();
        let (server_id, server_key) = keys();
        let (untrusted, _) = keys();
        let (client, server) = sockets().await;
        let pins = PeerPins::new(vec![if wrong_server { client_id } else { untrusted }]).unwrap();
        let (connected, accepted) = tokio::join!(
            connect(
                client,
                client_key,
                if wrong_server { untrusted } else { server_id },
                deadline()
            ),
            accept(server, server_key, pins, deadline()),
        );
        assert!(connected.is_err());
        assert!(accepted.is_err());
    }
}

#[tokio::test]
async fn expired_handshake_deadline_does_not_leave_a_live_authenticated_channel() {
    let (identity, key) = keys();
    let (client, server) = sockets().await;
    let immediate = tokio::time::Instant::now();
    assert!(
        connect(client, key.clone(), identity, immediate)
            .await
            .is_err()
    );
    assert!(
        accept(
            server,
            key,
            PeerPins::new(vec![identity]).unwrap(),
            immediate
        )
        .await
        .is_err()
    );
}

#[test]
fn ambiguous_identity_or_public_key_enrollment_is_rejected() {
    let (first, _) = keys();
    let (second, _) = keys();
    assert!(PeerPins::new(vec![first, second]).is_ok());
    assert!(PeerPins::new(vec![first, first]).is_err());
    let duplicate_key = yolo_hub_delegation::PeerIdentity::new(second.hub(), *first.key()).unwrap();
    assert!(PeerPins::new(vec![first, duplicate_key]).is_err());
}
