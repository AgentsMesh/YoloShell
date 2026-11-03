use super::fixtures::{constants::*, *};
use crate::{RevisionConflict, constants::*};
use yolo_hub_management_protocol::CONFIGURATION_REVISION_STEP;

#[test]
fn successful_cas_increments_once_and_exact_replay_returns_identical_state_without_writing() {
    let fixture = Fixture::new();
    let mut store = fixture.open(None).unwrap();
    let original = store.current().unwrap();
    let request = change(&original);
    assert_eq!(store.check(&request).unwrap(), original);
    let committed = store.apply(request.clone()).unwrap();
    assert_eq!(
        committed.revision,
        original.revision + CONFIGURATION_REVISION_STEP
    );
    assert_eq!(committed.last_change, request.request_id);
    assert_eq!(committed.configuration, request.configuration);
    let bytes = fixture.bytes();
    assert_eq!(store.apply(request).unwrap(), committed);
    assert_eq!(fixture.bytes(), bytes);
    let next = change(&committed);
    assert!(store.apply(next).is_ok());
}

#[test]
fn stale_or_mutated_replays_conflict_without_poisoning_the_store() {
    let fixture = Fixture::new();
    let mut store = fixture.open(None).unwrap();
    let request = change(&store.current().unwrap());
    let current = store.apply(request.clone()).unwrap();
    let mutations: &[fn(&mut yolo_hub_management_protocol::ConfigurationChange)] = &[
        |r| r.request_id = uuid::Uuid::new_v4(),
        |r| r.expected_revision += CONFIGURATION_REVISION_STEP,
        |r| r.configuration = yolo_hub_federation_settings::Configuration::default(),
    ];
    let before = fixture.bytes();
    for mutate in mutations {
        let mut invalid = request.clone();
        mutate(&mut invalid);
        let error = store.apply(invalid).unwrap_err();
        assert!(error.downcast_ref::<RevisionConflict>().is_some());
        assert_eq!(error.to_string(), REVISION_CONFLICT);
    }
    assert_eq!(fixture.bytes(), before);
    assert_eq!(store.current().unwrap(), current);
}

#[test]
fn revision_overflow_fails_without_wrapping_or_rewriting_the_record() {
    let fixture = Fixture::new();
    let store = fixture.open(None).unwrap();
    let mut record = fixture.record();
    record.state.revision = u64::MAX;
    drop(store);
    fixture.replace(&record);
    let mut store = fixture.open(None).unwrap();
    let state = store.current().unwrap();
    let bytes = fixture.bytes();
    assert_eq!(
        store.apply(change(&state)).unwrap_err().to_string(),
        REVISION_EXHAUSTED
    );
    assert_eq!(store.current().unwrap(), state);
    assert_eq!(fixture.bytes(), bytes);
}

#[test]
fn invalid_proposal_is_rejected_before_mutation_without_poisoning_the_owner() {
    let fixture = Fixture::new();
    let mut store = fixture.open(None).unwrap();
    let state = store.current().unwrap();
    let mut request = change(&state);
    request.configuration = invalid_configuration(fixture.identity);
    assert!(store.apply(request).is_err());
    assert_eq!(store.current().unwrap(), state);
    assert!(store.apply(change(&state)).is_ok());
}

#[test]
fn replacement_between_revision_check_and_commit_is_a_nonrecoverable_cas_failure() {
    let fixture = Fixture::new();
    let mut store = fixture.open(None).unwrap();
    let request = change(&store.current().unwrap());
    let mut next = store.check(&request).unwrap();
    next.revision += CONFIGURATION_REVISION_STEP;
    next.last_change = request.request_id;
    next.configuration = request.configuration;
    let mut replacement = fixture.record();
    replacement.state.configuration.listen = Some(std::net::SocketAddr::new(
        std::net::Ipv4Addr::LOCALHOST.into(),
        OTHER_PORT,
    ));
    fixture.replace(&replacement);
    let bytes = fixture.bytes();
    assert!(store.write(next).is_err());
    assert_eq!(fixture.bytes(), bytes);
    assert_eq!(store.current().unwrap_err().to_string(), LOST);
}
