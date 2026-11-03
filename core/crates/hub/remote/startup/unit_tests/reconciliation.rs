use crate::{configuration::plan, reconcile::confirmed};
use yolo_hub_remote_startup_test_support::request;
#[path = "../test_support/fixture.rs"]
mod fixture;

#[test]
fn unknown_configuration_is_confirmed_only_by_the_exact_applied_revision_and_document() {
    let request = request();
    let before = fixture::snapshot();
    let change = plan(&before, &request).unwrap().unwrap();
    assert!(!confirmed(&before, &change));
    let mut applied = before;
    applied.state.configuration = change.configuration.clone();
    applied.state.revision =
        change.expected_revision + yolo_hub_management_protocol::CONFIGURATION_REVISION_STEP;
    applied.state.last_change = change.request_id;
    assert!(confirmed(&applied, &change));
    let mut other = applied.clone();
    other.applied = false;
    assert!(!confirmed(&other, &change));
    let mut other = applied.clone();
    other.state.last_change = uuid::Uuid::new_v4();
    assert!(!confirmed(&other, &change));
    let mut other = applied.clone();
    other.state.revision += 1;
    assert!(!confirmed(&other, &change));
    let mut other = applied;
    other.state.configuration.peers.clear();
    assert!(!confirmed(&other, &change));
    let mut overflow = change;
    overflow.expected_revision = u64::MAX;
    assert!(!confirmed(&other, &overflow));
}
