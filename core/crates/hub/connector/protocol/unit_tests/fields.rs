use crate::{constants::*, fixture::Fixture};
use yolo_hub_connector_protocol::*;
use yolo_hub_identity::HubId;

#[test]
fn enrollment_and_activation_keep_agent_and_terminal_bound_to_one_hub() {
    let fixture = Fixture::new();
    fixture.enrollment.validate().unwrap();
    fixture.activation.validate().unwrap();
    fixture.enrollment.scope.validate_for(fixture.hub).unwrap();
    assert_eq!(
        fixture.enrollment.scope.validate_for(HubId::new()),
        Err(ConnectorError::InvalidRequest)
    );
    let mut enrollment = fixture.enrollment.clone();
    enrollment.agent.hub_id = HubId::new();
    assert!(enrollment.validate().is_err());
    let mut activation = fixture.activation;
    activation.terminal.address.hub_id = HubId::new();
    assert!(activation.validate().is_err());
    for status in [
        EnrollmentStatus::Enabled,
        EnrollmentStatus::Revoked,
        EnrollmentStatus::Superseded,
    ] {
        let mut summary = fixture.summary(status);
        summary.validate().unwrap();
        let wire = serde_json::to_vec(&summary).unwrap();
        assert_eq!(
            serde_json::from_slice::<EnrollmentSummary>(&wire).unwrap(),
            summary
        );
        summary.scope.terminal.address.hub_id = HubId::new();
        assert!(summary.validate().is_err());
        assert!(
            serde_json::from_value::<EnrollmentSummary>(serde_json::to_value(summary).unwrap())
                .is_err()
        );
    }
}

#[test]
fn zero_revisions_and_invalid_activation_sequences_fail_on_decode() {
    let fixture = Fixture::new();
    let mut enrollment = fixture.enrollment.clone();
    enrollment.revision = 0;
    assert_eq!(enrollment.validate(), Err(ConnectorError::InvalidRequest));
    assert!(
        serde_json::from_value::<Enrollment>(serde_json::to_value(enrollment).unwrap()).is_err()
    );
    let mut summary = fixture.summary(EnrollmentStatus::Enabled);
    summary.revision = 0;
    assert!(summary.validate().is_err());
    for sequence in [0, u64::MAX] {
        let mut activation = fixture.activation;
        activation.next_sequence = sequence;
        assert_eq!(activation.validate(), Err(ConnectorError::Sequence));
        assert!(
            serde_json::from_value::<Activation>(serde_json::to_value(activation).unwrap())
                .is_err()
        );
    }
    let mut boundary = fixture.activation;
    boundary.next_sequence = MAX_PUBLICATION_SEQUENCE;
    assert_eq!(boundary.validate(), Ok(()));
    let mut wire = serde_json::to_value(fixture.enrollment).unwrap();
    wire[UNKNOWN_FIELD] = true.into();
    assert!(serde_json::from_value::<Enrollment>(wire).is_err());
}

#[test]
fn error_codes_are_closed_serializable_and_have_safe_descriptions() {
    for code in [
        ConnectorError::UnsupportedSchema,
        ConnectorError::InvalidRequest,
        ConnectorError::Unauthorized,
        ConnectorError::NotFound,
        ConnectorError::Conflict,
        ConnectorError::StaleGeneration,
        ConnectorError::Sequence,
        ConnectorError::Unavailable,
        ConnectorError::Capacity,
        ConnectorError::OutcomeUnknown,
        ConnectorError::Internal,
    ] {
        let decoded =
            serde_json::from_slice::<ConnectorError>(&serde_json::to_vec(&code).unwrap()).unwrap();
        assert_eq!(decoded, code);
        assert!(!code.to_string().is_empty());
        assert!(std::error::Error::source(&code).is_none());
    }
    assert!(serde_json::from_value::<ConnectorError>(serde_json::json!(UNKNOWN_TEXT)).is_err());
}
