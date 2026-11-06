use crate::configuration::{plan, verify_images};
use yolo_hub_federation_settings::{ExportPolicy, Peer, PeerAddress};
use yolo_remote_provisioning_protocol::RemoteBootstrapRequest;
use yolo_hub_remote_startup_test_support::*;
#[path = "../test_support/fixture.rs"]
mod fixture;

#[test]
fn new_enrollment_preserves_existing_peers_and_only_exports_requested_origin_rights() {
    let mut snapshot = fixture::snapshot();
    let other = Peer {
        identity: peer(FOREIGN_HUB, FOREIGN_KEY),
        dial: None,
        consume: false,
        exports: vec![],
    };
    snapshot.state.configuration.peers.push(other.clone());
    let request = request();
    let change = plan(&snapshot, &request).unwrap().unwrap();
    assert_eq!(change.request_id, request.fields().request_id);
    assert_eq!(change.expected_revision, snapshot.state.revision);
    assert_eq!(change.configuration.peers[0], other);
    let added = change
        .configuration
        .peer(request.fields().consumer.hub())
        .unwrap();
    assert!(!added.consume && added.dial.is_none());
    assert_eq!(
        added.exports,
        vec![ExportPolicy {
            origin: snapshot.state.identity.hub(),
            rights: request.fields().rights.clone(),
            remaining_reexports: request.fields().remaining_reexports
        }]
    );
    snapshot.state.configuration = change.configuration;
    snapshot.peer_socket = Some("/run/user/1000/yolo/hub/peer.sock".into());
    assert!(plan(&snapshot, &request).unwrap().is_none());
}

#[test]
fn existing_exports_and_consume_direction_are_not_silently_changed() {
    let request = request();
    let mut snapshot = fixture::snapshot();
    snapshot
        .state
        .configuration
        .trusted_origins
        .push(peer(FOREIGN_HUB, FOREIGN_KEY));
    let existing = ExportPolicy {
        origin: peer(FOREIGN_HUB, FOREIGN_KEY).hub(),
        rights: request.fields().rights.clone(),
        remaining_reexports: 0,
    };
    snapshot.state.configuration.peers.push(Peer {
        identity: request.fields().consumer,
        dial: Some(
            PeerAddress::profile(
                "existing-profile",
                "/run/user/1000/yolo/hub/peer.sock".into(),
            )
            .unwrap(),
        ),
        consume: true,
        exports: vec![existing.clone()],
    });
    let change = plan(&snapshot, &request).unwrap().unwrap();
    let consumer = change
        .configuration
        .peer(request.fields().consumer.hub())
        .unwrap();
    assert!(consumer.consume);
    assert_eq!(consumer.exports[0], existing);
    snapshot.state.configuration = change.configuration;
    snapshot.peer_socket = Some("/run/user/1000/yolo/hub/peer.sock".into());
    snapshot.state.configuration.peers[0].exports[1].remaining_reexports += 1;
    assert!(plan(&snapshot, &request).is_err());
}

#[test]
fn pin_socket_and_apply_conflicts_fail_without_mutating_input() {
    let request = request();
    let original = fixture::snapshot();
    let mut snapshot = original.clone();
    snapshot.applied = false;
    assert!(plan(&snapshot, &request).is_err());
    let mut snapshot = original.clone();
    snapshot.peer_socket = None;
    assert!(plan(&snapshot, &request).is_err());
    let mut snapshot = original.clone();
    snapshot.state.identity = peer(FOREIGN_HUB, FOREIGN_KEY);
    assert!(plan(&snapshot, &request).is_err());
    let mut snapshot = original.clone();
    snapshot.state.configuration.peers.push(Peer {
        identity: peer(CONSUMER_HUB, FOREIGN_KEY),
        dial: None,
        consume: false,
        exports: vec![],
    });
    assert!(plan(&snapshot, &request).is_err());
    assert_eq!(
        original.state.configuration,
        yolo_hub_federation_settings::Configuration::default()
    );
}

#[test]
fn first_enrollment_accepts_public_identity_but_rejects_self_and_mismatched_images() {
    let mut fields = request().fields().clone();
    fields.expected_peer = None;
    let request = RemoteBootstrapRequest::new(fields.clone()).unwrap();
    assert!(plan(&fixture::snapshot(), &request).is_ok());
    fields.consumer = peer(REMOTE_HUB, CONSUMER_KEY);
    assert!(
        plan(
            &fixture::snapshot(),
            &RemoteBootstrapRequest::new(fields).unwrap()
        )
        .is_err()
    );
    let build = receipt(&request).build;
    verify_images(&build, &request).unwrap();
    for fault in super::image_faults::faults() {
        let error = verify_images(&fault.changed(&build), &request).unwrap_err();
        assert_eq!(
            error.downcast_ref::<yolo_remote_provisioning_protocol::BootstrapError>(),
            Some(&yolo_remote_provisioning_protocol::BootstrapError::UpgradeRequired),
            "{fault:?}"
        );
    }
}
