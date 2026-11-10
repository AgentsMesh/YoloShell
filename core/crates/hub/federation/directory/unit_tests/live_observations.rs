use crate::{exports::*, fixtures::directory, history_support::resign, support::*};
use yolo_hub_agent_protocol::{Freshness, Reachability};
use yolo_hub_peer_protocol::FIRST_REVISION;
use yolo_hub_terminal_protocol::RuntimeStatus;

#[test]
fn catalogue_expiry_marks_resources_stale_while_valid_grants_keep_the_hub_route_exportable() {
    let fixture = directory::fixture(None);
    let mut book = book(&fixture);
    let link = install(&mut book, &fixture.relay, vec![fixture.export.clone()]);
    let initial = book.observations(NOW).unwrap();
    let agent = initial.first().unwrap().agents.first().unwrap().fields();
    assert_eq!(agent.freshness, Freshness::Current);
    assert_eq!(agent.reachability, Reachability::Reachable);
    let mut expired = fixture.export.clone();
    let mut payload = expired.catalogue.payload.clone();
    payload.revision += STEP;
    payload.expires_at_ms = NOW;
    expired.catalogue = resign(payload, &fixture.origin);
    book.install(
        fixture.relay.identity,
        link,
        FIRST_REVISION + STEP,
        vec![expired.clone()],
        NOW,
    )
    .unwrap();
    assert_exports(&book, vec![expired], NOW);
    let observations = book.observations(NOW).unwrap();
    let view = observations.first().unwrap();
    assert_eq!(
        view.terminals
            .first()
            .unwrap()
            .fields()
            .runtime
            .unwrap()
            .status,
        RuntimeStatus::Unreachable
    );
    let agent = view.agents.first().unwrap().fields();
    assert_eq!(agent.freshness, Freshness::Stale);
    assert_eq!(agent.reachability, Reachability::Unreachable);
    assert!(book.hubs(NOW).unwrap().first().unwrap().reachable);
}
