use super::fixtures::{self, constants::*};
use crate::errors::map_error;
use yolo_client::ClientError;
use yolo_hub_terminal_protocol::PublicError;
use yolo_runtime::lease::constants::code as lease_code;
use yolo_runtime::protocol::{ProtocolError, error_code};

#[test]
fn remote_error_mapping_preserves_conflict_unknown_and_unreachable_distinctions() {
    for (code, expected) in [
        (error_code::NOT_FOUND, PublicError::NotFound),
        (error_code::UNAUTHORIZED, PublicError::Unauthorized),
        (error_code::IDENTITY_MISMATCH, PublicError::TargetMismatch),
        (error_code::CONFLICT, PublicError::Conflict),
        (error_code::LEASE_CONFLICT, PublicError::Conflict),
        (error_code::RECREATE_REQUIRED, PublicError::RecreateRequired),
        (error_code::INVALID_DIRECTORY, PublicError::InvalidDirectory),
        (lease_code::HELD_BY_OTHER, PublicError::Conflict),
        (lease_code::STALE_GENERATION, PublicError::Conflict),
        (lease_code::LEASE_EXPIRED, PublicError::Conflict),
        (lease_code::NOT_HOLDER, PublicError::Conflict),
        (
            lease_code::CONTROLLER_INSTANCE_MISMATCH,
            PublicError::Conflict,
        ),
        (lease_code::SESSION_NOT_RUNNING, PublicError::Conflict),
        (error_code::UNSUPPORTED_METHOD, PublicError::Unsupported),
        (
            error_code::DESTRUCTIVE_SESSION_UNAVAILABLE,
            PublicError::Unsupported,
        ),
        (
            error_code::AGENT_RECOVERY_UNAVAILABLE,
            PublicError::Unsupported,
        ),
        (error_code::INVALID_REQUEST, PublicError::InvalidRequest),
        (error_code::DELIVERY_UNKNOWN, PublicError::OutcomeUnknown),
        (error_code::CLEANUP_DEFERRED, PublicError::OutcomeUnknown),
        (
            error_code::PROTOCOL_INCOMPATIBLE,
            PublicError::OutcomeUnknown,
        ),
        (error_code::WORKER_UNREACHABLE, PublicError::Unreachable),
        (error_code::WORKER_SHUTTING_DOWN, PublicError::Unreachable),
        (UNKNOWN_ERROR, PublicError::Internal),
    ] {
        assert_eq!(
            map_error(ClientError::Remote {
                code: code.into(),
                message: MESSAGE.into(),
                retryable: false,
                details: None
            }),
            expected
        );
    }
}

#[test]
fn local_error_mapping_never_claims_uncertain_mutations_failed_before_delivery() {
    assert_eq!(
        map_error(ClientError::Protocol(ProtocolError::OutcomeUnknown)),
        PublicError::OutcomeUnknown
    );
    assert_eq!(
        map_error(ClientError::Deadline),
        PublicError::OutcomeUnknown
    );
    assert_eq!(
        map_error(ClientError::Protocol(ProtocolError::ConnectionPoisoned)),
        PublicError::OutcomeUnknown
    );
    assert_eq!(
        map_error(ClientError::RuntimeUnavailable {
            terminal_id: fixtures::target().address.terminal_id.as_uuid()
        }),
        PublicError::Unreachable
    );
    assert_eq!(
        map_error(ClientError::IncompleteCatalogInventory),
        PublicError::Internal
    );
}
