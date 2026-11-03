use crate::{fixtures::Fixture, support::*};
use yolo_hub_delegation::{DelegationChain, SignedGrant};
use yolo_hub_peer_protocol::{ExportedOrigin, Purpose, Signed};

pub fn empty_export(fixture: &Fixture) -> ExportedOrigin {
    let mut catalogue = fixture.export.catalogue.payload.clone();
    catalogue.terminals.clear();
    catalogue.agents.clear();
    ExportedOrigin {
        catalogue: Signed::new(catalogue, Purpose::Catalogue, &fixture.origin).unwrap(),
        authority: fixture.export.authority.clone(),
        reachable: true,
    }
}

pub fn direct_export(fixture: &Fixture) -> ExportedOrigin {
    let direct = SignedGrant::issue(
        root_claims(&fixture.origin, &fixture.entry),
        &fixture.origin,
    )
    .unwrap();
    ExportedOrigin {
        authority: DelegationChain::new(vec![direct]).unwrap(),
        ..fixture.export.clone()
    }
}
