use super::durable_state_validation_tests::durable_snapshot;
use super::invocation_set_tests::parked_pair;
use super::support::{Fixture, binding};

#[test]
fn parked_invocation_cannot_mutate_binding_before_publication_rejection() {
    let fixture = Fixture::new();
    let pair = parked_pair(&fixture);
    fixture
        .lease
        .write_json(&fixture.binding_path(), &binding(&pair.second))
        .unwrap();
    let before = durable_snapshot(&fixture);

    let error = fixture
        .store
        .publish_binding_state_in_namespace(
            &fixture.lease,
            &fixture.binding_path(),
            pair.first_authentication,
            &pair.first,
            false,
            |_| Ok(()),
        )
        .unwrap_err();

    assert!(
        error
            .to_string()
            .contains(super::super::constants::AUTHORITY_AUTHENTICATION_MISMATCH)
    );
    assert_eq!(durable_snapshot(&fixture), before);
}
