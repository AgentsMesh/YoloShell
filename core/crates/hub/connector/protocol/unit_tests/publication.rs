use crate::{constants::*, fixture::Fixture};
use yolo_hub_agent_protocol::{Activity, Lifecycle};
use yolo_hub_connector_protocol::*;
use yolo_hub_identity::HubId;

#[test]
fn reported_state_keeps_lifecycle_and_activity_independent_but_exit_cannot_claim_work() {
    let fixture = Fixture::new();
    for lifecycle in [
        Lifecycle::Unknown {},
        Lifecycle::Starting {},
        Lifecycle::Running {},
        Lifecycle::Exited { exit_code: None },
    ] {
        for activity in [
            Activity::Unknown,
            Activity::Idle,
            Activity::Working,
            Activity::WaitingForInput,
            Activity::WaitingForApproval,
        ] {
            let value = ReportedState {
                lifecycle,
                activity,
                working_directory: None,
            };
            let valid = !matches!(lifecycle, Lifecycle::Exited { .. })
                || matches!(activity, Activity::Unknown | Activity::Idle);
            assert_eq!(value.validate().is_ok(), valid);
            assert_eq!(
                serde_json::from_value::<ReportedState>(serde_json::to_value(&value).unwrap())
                    .is_ok(),
                valid
            );
        }
    }
    let encoded = serde_json::to_vec(&fixture.publication.state).unwrap();
    assert_eq!(
        serde_json::from_slice::<ReportedState>(&encoded).unwrap(),
        fixture.publication.state
    );
}

#[test]
fn publication_and_ack_reject_zero_reserved_sequence_and_mismatched_origin() {
    let fixture = Fixture::new();
    for sequence in [
        0,
        FIRST_PUBLICATION_SEQUENCE,
        MAX_PUBLICATION_SEQUENCE,
        u64::MAX,
    ] {
        let mut publication = fixture.publication.clone();
        publication.sequence = sequence;
        let valid = sequence != 0 && sequence != u64::MAX;
        assert_eq!(publication.validate().is_ok(), valid);
        assert_eq!(
            serde_json::from_value::<Publication>(serde_json::to_value(publication).unwrap())
                .is_ok(),
            valid
        );
        let ack = PublicationAck {
            sequence,
            ..fixture.ack
        };
        assert_eq!(ack.validate().is_ok(), valid);
        assert_eq!(
            serde_json::from_value::<PublicationAck>(serde_json::to_value(ack).unwrap()).is_ok(),
            valid
        );
    }
    let mut wrong = fixture.publication;
    wrong.terminal.address.hub_id = HubId::new();
    assert_eq!(wrong.validate(), Err(ConnectorError::InvalidRequest));
}

#[test]
fn reporting_cannot_inject_capabilities_custody_or_hook_provenance() {
    let fixture = Fixture::new();
    for field in FORBIDDEN_STATE_FIELDS {
        let mut state = serde_json::to_value(&fixture.publication.state).unwrap();
        state[*field] = true.into();
        assert!(serde_json::from_value::<ReportedState>(state).is_err());
        let mut publication = serde_json::to_value(&fixture.publication).unwrap();
        publication[*field] = true.into();
        assert!(serde_json::from_value::<Publication>(publication).is_err());
    }
    let mut state = serde_json::to_value(fixture.publication.state).unwrap();
    state[field::WORKING_DIRECTORY] = UNKNOWN_TEXT.into();
    assert!(serde_json::from_value::<ReportedState>(state).is_err());
}
