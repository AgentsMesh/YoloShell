use crate::{
    fixtures::{self, roundtrip},
    query_support::*,
};
use uuid::Uuid;
use yolo_hub_identity::HubId;
use yolo_hub_terminal_protocol::*;

#[test]
fn querying_creation_preserves_original_wire_parameters_without_exposing_debug_authority() {
    let query = request();
    assert_eq!(
        serde_json::to_value(&query).unwrap(),
        serde_json::to_value(query.creation()).unwrap()
    );
    roundtrip(&query);
    let debug = format!("{query:?}");
    assert!(debug.contains(QUERY_REDACTED));
    let fields = query.creation().fields();
    for private in [
        fields.creation_token.to_string(),
        fields.request_id.to_string(),
        fields.cwd.clone(),
        fields.name.clone().unwrap(),
    ] {
        assert!(!debug.contains(&private));
    }
}

#[test]
fn every_creation_outcome_preserves_its_exact_status_and_token() {
    let token = request().creation().fields().creation_token;
    for outcome in outcomes() {
        let result = result(outcome.clone());
        assert_eq!(result.creation_token(), token);
        assert_eq!(result.outcome(), &outcome);
        assert_eq!(
            result.validate_for(token, fixtures::address().hub_id),
            Ok(())
        );
        assert_eq!(
            result.validate_for(Uuid::new_v4(), fixtures::address().hub_id),
            Err(PublicError::InvalidRequest)
        );
        assert_eq!(
            result.validate_for(Uuid::nil(), fixtures::address().hub_id),
            Err(PublicError::InvalidRequest)
        );
        let addressed = matches!(
            outcome,
            CreationQueryOutcome::Ready { .. }
                | CreationQueryOutcome::Finalized { .. }
                | CreationQueryOutcome::Survivor { .. }
                | CreationQueryOutcome::Archived { .. }
        );
        assert_eq!(
            result.validate_for(token, HubId::new()),
            if addressed {
                Err(PublicError::OriginMismatch)
            } else {
                Ok(())
            }
        );
        roundtrip(&result);
        let debug = format!("{result:?}");
        assert!(debug.contains(QUERY_REDACTED));
        assert!(!debug.contains(&token.to_string()));
        assert!(!debug.contains(&creation().creation_capability.as_uuid().to_string()));
    }
    assert_eq!(
        CreationQueryResult::new(Uuid::nil(), CreationQueryOutcome::Missing {}),
        Err(PublicError::InvalidRequest)
    );
}
