use crate::{constants::*, types};
use yolo_hub_agent_protocol::{CommandOutcome, CommandReceipt, FailureCode};
use yolo_hub_relay_control_protocol::{NotSentReason, Outcome, Request};

pub(super) fn decode(value: &types::Outcome, request: &Request) -> Result<Outcome, ()> {
    match value.kind {
        UNKNOWN => Ok(Outcome::Unknown {}),
        NOT_SENT => Ok(Outcome::NotSent { reason: match value.reason {
            NO_WRITER => NotSentReason::NoWriter, NOT_SENT_EXPIRED => NotSentReason::Expired,
            TARGET_CHANGED => NotSentReason::TargetChanged, UNAVAILABLE => NotSentReason::Unavailable,
            _ => return Err(()),
        }}),
        RECEIPT => {
            let result = match value.state {
                ACCEPTED => CommandOutcome::Accepted {}, DELIVERED => CommandOutcome::Delivered {},
                CONFIRMED => CommandOutcome::Confirmed {}, RECEIPT_UNKNOWN => CommandOutcome::Unknown {},
                FAILED => CommandOutcome::Failed { code: match value.failure {
                    PERMISSION_DENIED => FailureCode::PermissionDenied, PRECONDITIONS_CHANGED => FailureCode::PreconditionsChanged,
                    UNSUPPORTED => FailureCode::Unsupported, EXPIRED => FailureCode::Expired,
                    ADAPTER_REJECTED => FailureCode::AdapterRejected, _ => return Err(()),
                }},
                _ => return Err(()),
            };
            let fields = request.fields().request.fields();
            let mut receipt = CommandReceipt::accepted(fields.request_id, fields.target).map_err(|_| ())?;
            receipt.advance(result).map_err(|_| ())?;
            Ok(Outcome::Receipt { receipt })
        }
        _ => Err(()),
    }
}

pub(super) fn encode(value: Outcome) -> types::Outcome {
    let mut result = types::Outcome::default();
    match value {
        Outcome::Unknown {} => result.kind = UNKNOWN,
        Outcome::NotSent { reason } => {
            result.kind = NOT_SENT;
            result.reason = match reason {
                NotSentReason::NoWriter => NO_WRITER, NotSentReason::Expired => NOT_SENT_EXPIRED,
                NotSentReason::TargetChanged => TARGET_CHANGED, NotSentReason::Unavailable => UNAVAILABLE,
            };
        }
        Outcome::Receipt { receipt } => {
            result.kind = RECEIPT;
            result.state = match receipt.outcome() {
                CommandOutcome::Accepted {} => ACCEPTED, CommandOutcome::Delivered {} => DELIVERED,
                CommandOutcome::Confirmed {} => CONFIRMED, CommandOutcome::Unknown {} => RECEIPT_UNKNOWN,
                CommandOutcome::Failed { code } => {
                    result.failure = match code {
                        FailureCode::PermissionDenied => PERMISSION_DENIED, FailureCode::PreconditionsChanged => PRECONDITIONS_CHANGED,
                        FailureCode::Unsupported => UNSUPPORTED, FailureCode::Expired => EXPIRED,
                        FailureCode::AdapterRejected => ADAPTER_REJECTED,
                    };
                    FAILED
                }
            };
        }
    }
    result
}
