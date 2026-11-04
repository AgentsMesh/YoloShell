use crate::{
    fixtures::{constants::*, reject_field},
    query_support::*,
};
use serde_json::Value;
use yolo_hub_terminal_protocol::*;

#[test]
fn query_requests_and_results_reject_noncanonical_tokens_and_extra_fields() {
    let request = request();
    for field in [field::CREATION_TOKEN, field::REQUEST_ID] {
        for invalid in [NIL_ID, INVALID_ID, CAPABILITY_UPPER, CAPABILITY_COMPACT] {
            reject_field(&request, field, invalid.into());
        }
    }
    reject_field(&request, UNKNOWN_FIELD, true.into());
    for outcome in outcomes() {
        let result = result(outcome);
        reject_field(&result, UNKNOWN_FIELD, true.into());
        for invalid in [NIL_ID, INVALID_ID, CAPABILITY_UPPER, CAPABILITY_COMPACT] {
            reject_field(&result, field::CREATION_TOKEN, invalid.into());
        }
        let mut wire = serde_json::to_value(&result).unwrap();
        wire[OUTCOME_FIELD][UNKNOWN_FIELD] = true.into();
        assert!(serde_json::from_value::<CreationQueryResult>(wire).is_err());
        let mut wire = serde_json::to_value(&result).unwrap();
        wire[OUTCOME_FIELD][STATUS_FIELD] = UNKNOWN_VALUE.into();
        assert!(serde_json::from_value::<CreationQueryResult>(wire).is_err());
    }
}

#[test]
fn nonterminal_and_archived_outcomes_cannot_smuggle_creation_authority() {
    for outcome in [
        CreationQueryOutcome::Missing {},
        CreationQueryOutcome::Pending {},
        CreationQueryOutcome::Unknown {},
        CreationQueryOutcome::Archived {
            address: creation().address,
        },
    ] {
        let mut wire = serde_json::to_value(result(outcome)).unwrap();
        wire[OUTCOME_FIELD][CREATION_FIELD] = serde_json::to_value(creation()).unwrap();
        assert!(serde_json::from_value::<CreationQueryResult>(wire).is_err());
    }
}

#[test]
fn ready_finalized_and_survivor_require_a_valid_creation_receipt() {
    for outcome in [
        CreationQueryOutcome::Ready {
            creation: creation(),
        },
        CreationQueryOutcome::Finalized {
            creation: creation(),
        },
        CreationQueryOutcome::Survivor {
            creation: creation(),
        },
    ] {
        let valid = serde_json::to_value(result(outcome)).unwrap();
        let mut missing = valid.clone();
        missing[OUTCOME_FIELD]
            .as_object_mut()
            .unwrap()
            .remove(CREATION_FIELD);
        assert!(serde_json::from_value::<CreationQueryResult>(missing).is_err());
        let mut invalid = valid.clone();
        invalid[OUTCOME_FIELD][CREATION_FIELD][CAPABILITY_FIELD] = Value::String(NIL_ID.into());
        assert!(serde_json::from_value::<CreationQueryResult>(invalid).is_err());
        let mut invalid = valid;
        invalid[OUTCOME_FIELD][CREATION_FIELD][field::ADDRESS] = Value::Null;
        assert!(serde_json::from_value::<CreationQueryResult>(invalid).is_err());
    }
    let mut wire = serde_json::to_value(result(CreationQueryOutcome::Missing {})).unwrap();
    wire[OUTCOME_FIELD][STATUS_FIELD] = READY_STATUS.into();
    assert!(serde_json::from_value::<CreationQueryResult>(wire).is_err());
}
