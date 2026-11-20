#[path = "../test_support/constants.rs"]
mod constants;
#[path = "../test_support/fixtures.rs"]
mod fixtures;
use fixtures::*;
use yolo_hub_enrollment_store::{EnrollmentRecord, PreparedEnrollment};

#[test]
fn prepared_authority_and_completion_require_exact_request_pin_configuration_and_revision() {
    let original = prepared(false);
    assert!(!original.applied_to(&original.snapshot));
    assert!(original.applied_to(&applied(&original)));
    let mut changed = original.clone();
    changed.change.as_mut().unwrap().request_id = uuid::Uuid::new_v4();
    assert!(changed.validate().is_err());
    let mut changed = original.clone();
    changed.change.as_mut().unwrap().expected_revision += 1;
    assert!(changed.validate().is_err());
    let mut changed = original.clone();
    changed.change.as_mut().unwrap().configuration.peers.clear();
    assert!(changed.validate().is_err());
    let mut changed = applied(&original);
    changed.state.revision += 1;
    assert!(!original.applied_to(&changed));
    let mut record = EnrollmentRecord {
        prepared: original.clone(),
        completion: Some(completed(&original)),
    };
    record.validate().unwrap();
    record.completion = Some(completed(&prepared(false)));
    assert!(record.validate().is_err());
}

#[test]
fn no_change_evidence_requires_the_original_applied_state_and_explicit_fields() {
    let prepared = prepared(true);
    assert!(prepared.applied_to(&prepared.snapshot));
    let mut superseded = prepared.snapshot.clone();
    superseded.state.last_change = uuid::Uuid::new_v4();
    assert!(!prepared.applied_to(&superseded));
    let mut wire = serde_json::to_value(&prepared).unwrap();
    wire.as_object_mut()
        .unwrap()
        .remove(constants::CHANGE_FIELD);
    assert!(serde_json::from_value::<PreparedEnrollment>(wire).is_err());
    let record = EnrollmentRecord {
        prepared,
        completion: None,
    };
    let mut wire = serde_json::to_value(&record).unwrap();
    wire.as_object_mut()
        .unwrap()
        .remove(constants::COMPLETION_FIELD);
    assert!(serde_json::from_value::<EnrollmentRecord>(wire).is_err());
    let mut wire = serde_json::to_value(&record).unwrap();
    wire[constants::EXTRA_FIELD] = true.into();
    assert!(serde_json::from_value::<EnrollmentRecord>(wire).is_err());
}
