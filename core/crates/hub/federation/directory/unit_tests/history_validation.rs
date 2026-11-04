use crate::{fixtures::*, support::*};
use yolo_hub_delegation::{DelegationChain, SignedGrant};
use yolo_hub_federation_directory::OriginBook;
use yolo_hub_federation_settings::routing_limits;
use yolo_hub_peer_protocol::{ExportedOrigin, FIRST_REVISION, Purpose, Signed};

const SINGLE_ORIGIN: usize = 1;

#[test]
fn history_rejects_another_entry_identity_and_cannot_reset_an_active_directory() {
    let fixture = Fixture::new();
    let mut previous = OriginBook::new(
        fixture.entry.identity,
        vec![fixture.relay.identity, fixture.origin.identity],
        routing_limits(),
    )
    .unwrap();
    let link = uuid::Uuid::new_v4();
    previous.connect(fixture.relay.identity, link).unwrap();
    previous
        .install(
            fixture.relay.identity,
            link,
            FIRST_REVISION,
            vec![fixture.export.clone()],
            NOW,
        )
        .unwrap();
    let history = previous.history();
    assert!(previous.restore_history(history.clone()).is_err());
    assert!(
        previous
            .route_for(fixture.origin.identity.hub(), &observe(), NOW)
            .unwrap()
            .is_some()
    );
    let mut foreign = OriginBook::new(fixture.relay.identity, vec![], routing_limits()).unwrap();
    assert!(foreign.restore_history(history.clone()).is_err());
    assert!(foreign.hubs(NOW).unwrap().is_empty());
    let changed_key = yolo_hub_delegation::PeerIdentity::new(
        fixture.entry.identity.hub(),
        *fixture.origin.identity.key(),
    )
    .unwrap();
    let mut changed = OriginBook::new(changed_key, vec![], routing_limits()).unwrap();
    assert!(changed.restore_history(history).is_err());
}

#[test]
fn history_capacity_failure_is_atomic_and_does_not_relax_origin_limits() {
    let fixture = Fixture::new();
    let other = Identity::new();
    let mut previous = OriginBook::new(
        fixture.entry.identity,
        vec![
            fixture.relay.identity,
            fixture.origin.identity,
            other.identity,
        ],
        routing_limits(),
    )
    .unwrap();
    let mut catalogue = fixture.export.catalogue.payload.clone();
    catalogue.terminals.clear();
    let mut claims = root_claims(&other, &fixture.relay);
    claims.policy_epoch = catalogue.policy_epoch;
    let root = SignedGrant::issue(claims, &other).unwrap();
    let child = SignedGrant::issue(child_claims(&root, &fixture.entry), &fixture.relay).unwrap();
    let export = ExportedOrigin {
        catalogue: Signed::new(catalogue, Purpose::Catalogue, &other).unwrap(),
        authority: DelegationChain::new(vec![root, child]).unwrap(),
        reachable: true,
    };
    let link = uuid::Uuid::new_v4();
    previous.connect(fixture.relay.identity, link).unwrap();
    previous
        .install(
            fixture.relay.identity,
            link,
            FIRST_REVISION,
            vec![fixture.export.clone(), export],
            NOW,
        )
        .unwrap();
    let mut limits = routing_limits();
    limits.max_known_agents = std::num::NonZeroUsize::new(SINGLE_ORIGIN).unwrap();
    let mut next = OriginBook::new(fixture.entry.identity, vec![], limits).unwrap();
    assert!(next.restore_history(previous.history()).is_err());
    assert!(next.hubs(NOW).unwrap().is_empty());
    assert!(next.exportable(NOW).unwrap().is_empty());
}
