use uuid::Uuid;
use yolo_hub_agent_protocol::*;

use crate::fixtures::*;

const OUTCOMES: [CommandOutcome; 5] = [
    CommandOutcome::Accepted {},
    CommandOutcome::Delivered {},
    CommandOutcome::Confirmed {},
    CommandOutcome::Unknown {},
    CommandOutcome::Failed {
        code: FailureCode::AdapterRejected,
    },
];

#[test]
fn receipt_identity_and_every_transition_are_checked() {
    let target = target();
    let request_id = Uuid::new_v4();
    let accepted = CommandReceipt::accepted(request_id, target).unwrap();
    assert_eq!(accepted.request_id(), request_id);
    assert_eq!(accepted.target(), target);
    assert_eq!(accepted.outcome(), CommandOutcome::Accepted {});
    assert_eq!(
        CommandReceipt::accepted(Uuid::nil(), target),
        Err(ProtocolError::InvalidRequest)
    );
    for from in OUTCOMES {
        for to in OUTCOMES {
            let mut receipt = accepted;
            receipt.advance(from).unwrap();
            let allowed = from == to
                || match from {
                    CommandOutcome::Accepted {} => true,
                    CommandOutcome::Delivered {} | CommandOutcome::Unknown {} => {
                        to != CommandOutcome::Accepted {}
                    }
                    CommandOutcome::Confirmed {} | CommandOutcome::Failed { .. } => false,
                };
            assert_eq!(receipt.advance(to).is_ok(), allowed, "{from:?} -> {to:?}");
            assert_eq!(receipt.outcome(), if allowed { to } else { from });
            assert_eq!(round_trip(&receipt), receipt);
        }
    }
}

#[test]
fn failure_codes_round_trip_and_cannot_rewrite_confirmed_failure() {
    for code in [
        FailureCode::PermissionDenied,
        FailureCode::PreconditionsChanged,
        FailureCode::Unsupported,
        FailureCode::AdapterRejected,
        FailureCode::Expired,
    ] {
        let mut receipt = CommandReceipt::accepted(Uuid::new_v4(), target()).unwrap();
        receipt.advance(CommandOutcome::Failed { code }).unwrap();
        assert_eq!(round_trip(&receipt), receipt);
        assert_eq!(
            receipt.advance(CommandOutcome::Confirmed {}),
            Err(ProtocolError::InvalidTransition)
        );
    }
    let receipt = CommandReceipt::accepted(Uuid::new_v4(), target()).unwrap();
    let mut wire = serde_json::to_value(receipt).unwrap();
    wire[REQUEST_ID_FIELD] = Uuid::nil().to_string().into();
    assert!(serde_json::from_value::<CommandReceipt>(wire).is_err());
}

#[test]
fn rejection_is_bound_to_the_validated_request_and_never_promoted_to_execution() {
    let request =
        AgentControlRequest::new(request_fields(&AgentSnapshot::new(fields()).unwrap())).unwrap();
    for code in [
        FailureCode::Unsupported,
        FailureCode::PreconditionsChanged,
        FailureCode::PermissionDenied,
    ] {
        let mut receipt = CommandReceipt::rejected(&request, code);
        assert_eq!(receipt.request_id(), request.fields().request_id);
        assert_eq!(receipt.target(), request.fields().target);
        assert_eq!(receipt.outcome(), CommandOutcome::Failed { code });
        assert_eq!(round_trip(&receipt), receipt);
        for next in [
            CommandOutcome::Accepted {},
            CommandOutcome::Delivered {},
            CommandOutcome::Confirmed {},
            CommandOutcome::Unknown {},
        ] {
            assert_eq!(receipt.advance(next), Err(ProtocolError::InvalidTransition));
            assert_eq!(receipt.outcome(), CommandOutcome::Failed { code });
        }
    }
}
