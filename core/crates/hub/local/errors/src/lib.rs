use yolo_client::ClientError;
use yolo_hub_terminal_protocol::PublicError;
use yolo_lease::constants::code as lease_code;
use yolo_wire::error_code;

pub fn map_error(error: ClientError) -> PublicError {
    match error {
        ClientError::Remote { code, .. } => match code.as_str() {
            error_code::NOT_FOUND => PublicError::NotFound,
            error_code::UNAUTHORIZED => PublicError::Unauthorized,
            error_code::IDENTITY_MISMATCH => PublicError::TargetMismatch,
            error_code::INVALID_DIRECTORY => PublicError::InvalidDirectory,
            error_code::RECREATE_REQUIRED => PublicError::RecreateRequired,
            error_code::CONFLICT | error_code::LEASE_CONFLICT => PublicError::Conflict,
            lease_code::HELD_BY_OTHER
            | lease_code::STALE_GENERATION
            | lease_code::LEASE_EXPIRED
            | lease_code::NOT_HOLDER
            | lease_code::CONTROLLER_INSTANCE_MISMATCH
            | lease_code::SESSION_NOT_RUNNING => PublicError::Conflict,
            error_code::UNSUPPORTED_METHOD
            | error_code::DESTRUCTIVE_SESSION_UNAVAILABLE
            | error_code::AGENT_RECOVERY_UNAVAILABLE
            | error_code::WORKTREE_UNAVAILABLE => PublicError::Unsupported,
            error_code::WORKTREE_UNTRUSTED_REPOSITORY
            | error_code::WORKTREE_INVALID_REPOSITORY
            | error_code::WORKTREE_INVALID_BRANCH => PublicError::InvalidRequest,
            error_code::WORKTREE_TARGET_EXISTS
            | error_code::WORKTREE_NOT_REGISTERED
            | error_code::WORKTREE_GIT_FAILED => PublicError::Conflict,
            error_code::WORKTREE_TIMEOUT
            | error_code::WORKTREE_UNKNOWN
            | error_code::WORKTREE_IO_FAILED => PublicError::OutcomeUnknown,
            error_code::INVALID_REQUEST => PublicError::InvalidRequest,
            error_code::DELIVERY_UNKNOWN
            | error_code::CLEANUP_DEFERRED
            | error_code::PROTOCOL_INCOMPATIBLE => PublicError::OutcomeUnknown,
            error_code::WORKTREE_OUTPUT_TOO_LARGE => PublicError::Internal,
            error_code::WORKER_UNREACHABLE | error_code::WORKER_SHUTTING_DOWN => {
                PublicError::Unreachable
            }
            _ => PublicError::Internal,
        },
        ClientError::Deadline | ClientError::Protocol(_) => PublicError::OutcomeUnknown,
        ClientError::RuntimeUnavailable { .. } => PublicError::Unreachable,
        _ => PublicError::Internal,
    }
}
