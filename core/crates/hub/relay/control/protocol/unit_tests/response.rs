use crate::fixtures::*;
use yolo_hub_agent_protocol::{CommandOutcome, CommandReceipt, FailureCode};
use yolo_hub_relay_control_protocol::{
    NotSentReason, Outcome, ProtocolError, Response, ResponseFields, decode_response,
    encode_response,
};

#[test]
fn every_presend_reason_and_unknown_preserves_full_echo() {
    let request = request();
    for outcome in [
        Outcome::NotSent {
            reason: NotSentReason::NoWriter,
        },
        Outcome::NotSent {
            reason: NotSentReason::Expired,
        },
        Outcome::NotSent {
            reason: NotSentReason::TargetChanged,
        },
        Outcome::NotSent {
            reason: NotSentReason::Unavailable,
        },
        Outcome::Unknown {},
    ] {
        let response = Response::new(&request, outcome).unwrap();
        assert_eq!(response.fields().outcome, outcome);
        assert_eq!(response.validate_for(&request), Ok(()));
        assert_eq!(
            decode_response(&encode_response(&response).unwrap()),
            Ok(response)
        );
    }
}

#[test]
fn every_public_receipt_state_round_trips_without_reinterpretation() {
    let request = request();
    let mut outcomes = vec![
        CommandOutcome::Accepted {},
        CommandOutcome::Delivered {},
        CommandOutcome::Confirmed {},
        CommandOutcome::Unknown {},
    ];
    for code in [
        FailureCode::PermissionDenied,
        FailureCode::PreconditionsChanged,
        FailureCode::Unsupported,
        FailureCode::AdapterRejected,
        FailureCode::Expired,
    ] {
        outcomes.push(CommandOutcome::Failed { code });
    }
    for outcome in outcomes {
        let outcome = Outcome::Receipt {
            receipt: receipt(&request, outcome),
        };
        let response = Response::new(&request, outcome).unwrap();
        let fields = ResponseFields::from(response.clone());
        assert_eq!(Response::try_from(fields), Ok(response.clone()));
        assert_eq!(
            decode_response(&encode_response(&response).unwrap()),
            Ok(response)
        );
    }
}

#[test]
fn receipt_must_match_request_id_and_complete_agent_target() {
    let request = request();
    let control = request.fields().request.fields();
    let mut targets = vec![control.target];
    let mut changed = control.target;
    changed.execution_id = ALTERNATE_ID.parse().unwrap();
    targets.push(changed);
    changed = control.target;
    changed.address.agent_id = ALTERNATE_ID.parse().unwrap();
    targets.push(changed);
    changed = control.target;
    changed.address.hub_id = ALTERNATE_ID.parse().unwrap();
    targets.push(changed);
    for target in targets {
        let id = if target == control.target {
            alternate_uuid()
        } else {
            control.request_id
        };
        let receipt = CommandReceipt::accepted(id, target).unwrap();
        assert_eq!(
            Response::new(&request, Outcome::Receipt { receipt }),
            Err(ProtocolError::ReceiptMismatch)
        );
    }
}

#[test]
fn response_fields_cannot_bypass_echo_or_outcome_validation() {
    let request = request();
    let response = response(&request);
    let mut fields = response.fields().clone();
    fields.schema_version = UNSUPPORTED_SCHEMA;
    assert_eq!(
        Response::try_from(fields),
        Err(ProtocolError::UnsupportedSchema)
    );
    let mut fields = response.fields().clone();
    fields.outcome = Outcome::Receipt {
        receipt: CommandReceipt::accepted(
            alternate_uuid(),
            request.fields().request.fields().target,
        )
        .unwrap(),
    };
    assert_eq!(
        Response::try_from(fields),
        Err(ProtocolError::ReceiptMismatch)
    );
}
