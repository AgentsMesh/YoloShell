use crate::{BlockedReason, rejection};
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::ControlError;
use yolo_hub_terminal_protocol::PublicError;

#[test]
fn channel_loss_capacity_and_task_failure_only_schedule_another_read() {
    for error in [
        ControlError::NotConnected,
        ControlError::StaleGeneration,
        ControlError::Invalidated,
        ControlError::OperationLimit,
        ControlError::QueryLimit,
        ControlError::QueryTaskFailed,
        ControlError::Client(ClientError::Poisoned),
        ControlError::Client(ClientError::Remote(PublicError::Unreachable)),
        ControlError::Client(ClientError::Remote(PublicError::Internal)),
    ] {
        assert_eq!(rejection::error(&error), None);
    }
}

#[test]
fn unknown_rejection_and_invalid_responses_stay_blocked_without_private_error_text() {
    for (error, expected) in [
        (
            ClientError::Remote(PublicError::OutcomeUnknown),
            BlockedReason::Unknown,
        ),
        (
            ClientError::Remote(PublicError::Unauthorized),
            BlockedReason::Rejected(PublicError::Unauthorized),
        ),
        (
            ClientError::Validation(PublicError::InvalidRequest),
            BlockedReason::Rejected(PublicError::InvalidRequest),
        ),
        (
            ClientError::InvalidResponse(PublicError::OriginMismatch),
            BlockedReason::InvalidResponse,
        ),
        (
            ClientError::OutcomeUnknownResponse(PublicError::InvalidPayload),
            BlockedReason::InvalidResponse,
        ),
        (ClientError::TerminalStream, BlockedReason::InvalidResponse),
        (ClientError::Unsupported, BlockedReason::Unsupported),
        (ClientError::UnsupportedEndpoint, BlockedReason::Unsupported),
        (ClientError::InvalidEndpoint, BlockedReason::InvalidEndpoint),
        (ClientError::WrongLocalUser, BlockedReason::WrongLocalUser),
    ] {
        assert_eq!(
            rejection::error(&ControlError::Client(error)),
            Some(expected)
        );
        assert!(!expected.to_string().is_empty());
    }
}
