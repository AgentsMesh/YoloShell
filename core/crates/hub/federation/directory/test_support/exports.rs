use crate::{fixtures::Fixture, support::*};
use uuid::Uuid;
use yolo_hub_delegation::{DelegationChain, SignedGrant};
use yolo_hub_federation_directory::OriginBook;
use yolo_hub_federation_settings::routing_limits;
use yolo_hub_peer_protocol::{Catalogue, ExportedOrigin, FIRST_REVISION, Signed, route};
use yolo_hub_routing::Rights;

pub const ONE_PATH: usize = 1;
pub const TWO_PATHS: usize = 2;

pub fn export_via(
    origin: &Identity,
    entry: &Identity,
    intermediates: &[&Identity],
    catalogue: Signed<Catalogue>,
    rights: Rights,
) -> ExportedOrigin {
    let recipients: Vec<_> = intermediates.iter().copied().chain([entry]).collect();
    let mut claims = root_claims(origin, recipients[0]);
    claims.policy_epoch = catalogue.payload.policy_epoch;
    claims.rights = rights;
    let mut grants = vec![SignedGrant::issue(claims, origin).unwrap()];
    for pair in recipients.windows(TWO_PATHS) {
        let child = child_claims(grants.last().unwrap(), pair[1]);
        grants.push(SignedGrant::issue(child, pair[0]).unwrap());
    }
    ExportedOrigin {
        catalogue,
        authority: DelegationChain::new(grants).unwrap(),
        reachable: true,
    }
}

pub fn book(fixture: &Fixture) -> OriginBook {
    OriginBook::new(
        fixture.entry.identity,
        vec![fixture.origin.identity, fixture.relay.identity],
        routing_limits(),
    )
    .unwrap()
}

pub fn install(book: &mut OriginBook, peer: &Identity, exports: Vec<ExportedOrigin>) -> Uuid {
    let link = Uuid::new_v4();
    book.connect(peer.identity, link).unwrap();
    book.install(peer.identity, link, FIRST_REVISION, exports, NOW)
        .unwrap();
    link
}

pub fn assert_exports(book: &OriginBook, mut expected: Vec<ExportedOrigin>, now: u64) {
    expected.sort_by_key(|export| route(&export.authority));
    let actual: Vec<_> = book
        .exportable(now)
        .unwrap()
        .iter()
        .map(|export| (**export).clone())
        .collect();
    assert_eq!(actual, expected);
}
