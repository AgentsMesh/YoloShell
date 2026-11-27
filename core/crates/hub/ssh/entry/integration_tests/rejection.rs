use crate::support::*;
use std::sync::atomic::Ordering;
use yolo_hub_client::{ClientError, EntryConnector};
use yolo_hub_identity::HubId;
use yolo_hub_ssh_entry::SshEntry;

#[tokio::test]
async fn unexpected_hub_identity_is_rejected_before_any_network_connection() {
    let fixture = Fixture::start(Service::Echo).await;
    let entry = fixture.entry();
    assert!(matches!(
        entry.open(HubId::new(), deadline()).await,
        Err(ClientError::InvalidEndpoint)
    ));
    assert_eq!(fixture.counts.connections.load(Ordering::SeqCst), NO_COUNT);
}

#[tokio::test]
async fn wrong_outer_pin_hub_pin_socket_and_account_credentials_fail_closed() {
    for mismatch in [
        Mismatch::OuterPin,
        Mismatch::HubPin,
        Mismatch::Socket,
        Mismatch::AccountKey,
        Mismatch::ClientKey,
    ] {
        let fixture = Fixture::start(Service::Echo).await;
        let mut profile = fixture.profile.clone();
        let (wrong_public, wrong_key) = keys::keys();
        match mismatch {
            Mismatch::OuterPin => {
                profile.outer_host_key = wrong_key.public_key().to_openssh().unwrap()
            }
            Mismatch::HubPin => profile.hub.public_key = wrong_public,
            Mismatch::Socket => profile.remote_socket = WRONG_SOCKET.into(),
            _ => {}
        }
        let outer = if matches!(mismatch, Mismatch::AccountKey) {
            wrong_key.clone()
        } else {
            fixture.outer_key.clone()
        };
        let inner = if matches!(mismatch, Mismatch::ClientKey) {
            wrong_key
        } else {
            fixture.inner_key.clone()
        };
        let entry = SshEntry::new(profile.clone(), outer, inner).unwrap();
        let error = entry
            .open(profile.hub.hub_id, deadline())
            .await
            .err()
            .unwrap();
        let rendered = format!("{error:?} {error}");
        assert!(!rendered.contains(&profile.outer_host_key));
        assert!(!rendered.contains(SOCKET));
        assert!(!rendered.contains(USERNAME));
        assert_eq!(fixture.counts.mutations.load(Ordering::SeqCst), NO_COUNT);
        entry.close(deadline()).await.unwrap();
    }
}
#[derive(Clone, Copy)]
enum Mismatch {
    OuterPin,
    HubPin,
    Socket,
    AccountKey,
    ClientKey,
}
