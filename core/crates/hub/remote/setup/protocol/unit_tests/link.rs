use super::{constants::*, fixture::*};
use yolo_hub_federation_settings::{ExportPolicy, Peer, PeerAddress};
use yolo_hub_remote_setup_protocol::*;
use yolo_remote_provisioning_protocol::ProvisionOutcome;

#[test]
fn linking_preserves_existing_policy_and_is_noop_only_for_the_exact_ssh_route() {
    let request = request();
    let provision = ready(&request);
    let baseline = baseline(&request);
    let change = plan_link(&baseline, &request, &provision).unwrap().unwrap();
    assert_eq!(change.request_id, request.request.fields().request_id);
    assert_eq!(change.expected_revision, baseline.state.revision);
    let peer = change.configuration.peers.first().unwrap().clone();
    assert!(peer.consume && peer.exports.is_empty());
    let dial = peer.dial.as_ref().unwrap();
    assert_eq!(dial.profile_id(), Some(request.profile().unwrap().profile_id()));
    let ProvisionOutcome::Ready { enrollment, .. } = &provision.fields().outcome else {
        panic!()
    };
    assert_eq!(dial.socket(), enrollment.fields().peer_socket.as_path());
    let mut connected = baseline.clone();
    connected.state.configuration = change.configuration;
    assert!(configuration_contains_link(&connected, &provision).unwrap());
    assert!(
        plan_link(&connected, &request, &provision)
            .unwrap()
            .is_none()
    );
    let mut current = baseline;
    current.state.configuration.peers.push(Peer {
        dial: None,
        consume: false,
        exports: vec![ExportPolicy {
            origin: current.state.identity.hub(),
            rights: request.request.fields().bootstrap.fields().rights.clone(),
            remaining_reexports: 0,
        }],
        ..peer.clone()
    });
    let link = plan_link(&current, &request, &provision).unwrap().unwrap();
    assert_eq!(
        link.configuration.peers.len(),
        current.state.configuration.peers.len()
    );
    assert_eq!(
        link.configuration.peers[0].exports,
        current.state.configuration.peers[0].exports
    );
    current.state.configuration.peers[0].dial =
        Some(PeerAddress::profile(OTHER_PROFILE, OTHER_SOCKET.into()).unwrap());
    assert!(plan_link(&current, &request, &provision).is_err());
}

#[test]
fn planning_rejects_stale_revision_wrong_local_authority_and_unready_provision() {
    let request = request();
    let provision = ready(&request);
    let baseline = baseline(&request);
    let mut changed = baseline.clone();
    changed.state.revision += 1;
    assert!(plan_link(&changed, &request, &provision).is_err());
    let mut changed = baseline.clone();
    changed.applied = false;
    assert!(plan_link(&changed, &request, &provision).is_err());
    let mut changed = baseline.clone();
    changed.state.identity = request
        .request
        .fields()
        .bootstrap
        .fields()
        .expected_peer
        .unwrap();
    assert!(plan_link(&changed, &request, &provision).is_err());
    assert!(plan_link(&baseline, &request, &published(&request)).is_err());
    assert!(!configuration_contains_link(&baseline, &provision).unwrap());
}
