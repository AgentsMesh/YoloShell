use yolo_client::ClientError;
use yolo_lease::constants::code as lease_code;
use yolo_wire::{ProtocolError, error_code};
use yolo_runtime_protocol::NativeControlOutcome;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RpcStage {
    Submit,
    ReceiptBeforeRefresh,
    ReceiptAfterRefresh,
    Interrupt,
    Respond,
}
/// Static classifications deliberately cannot retain an error's arbitrary text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RpcResult {
    Missing,
    Delivered,
    NotDelivered,
    Unknown,
    CorrelationMismatch,
    Deadline,
    Protocol,
    Remote { code: Option<&'static str> },
    InvalidLocalHome,
    InvalidHostHome,
    HostProtocolMismatch,
    HostCapabilityUnavailable,
    HostHomeMismatch,
    RuntimeUnavailable,
    IncompleteCatalogInventory,
}
impl RpcResult {
    pub fn client_error(error: &ClientError) -> Self {
        match error {
            ClientError::Deadline => Self::Deadline,
            ClientError::Protocol(ProtocolError::ResponseMismatch) => Self::CorrelationMismatch,
            ClientError::Protocol(_) => Self::Protocol,
            ClientError::Remote { code, .. } => Self::Remote {
                code: KNOWN_REMOTE_CODES
                    .iter()
                    .copied()
                    .find(|known| *known == code.as_str()),
            },
            ClientError::InvalidLocalHome { .. } => Self::InvalidLocalHome,
            ClientError::InvalidHostHome { .. } => Self::InvalidHostHome,
            ClientError::HostProtocolMismatch { .. } => Self::HostProtocolMismatch,
            ClientError::HostCapabilityUnavailable { .. } => Self::HostCapabilityUnavailable,
            ClientError::HostHomeMismatch { .. } => Self::HostHomeMismatch,
            ClientError::RuntimeUnavailable { .. } => Self::RuntimeUnavailable,
            ClientError::IncompleteCatalogInventory => Self::IncompleteCatalogInventory,
        }
    }
}

impl From<NativeControlOutcome> for RpcResult {
    fn from(outcome: NativeControlOutcome) -> Self {
        match outcome {
            NativeControlOutcome::NotDelivered => Self::NotDelivered,
            NativeControlOutcome::Unknown => Self::Unknown,
            _ => Self::Delivered,
        }
    }
}

const KNOWN_REMOTE_CODES: &[&str] = &[
    error_code::CONFLICT,
    error_code::CLEANUP_DEFERRED,
    error_code::DELIVERY_UNKNOWN,
    error_code::DESTRUCTIVE_SESSION_UNAVAILABLE,
    error_code::AGENT_RECOVERY_UNAVAILABLE,
    error_code::HOST_REBOOTED,
    error_code::IDENTITY_MISMATCH,
    error_code::INTERNAL,
    error_code::INVALID_REQUEST,
    error_code::LEASE_CONFLICT,
    error_code::NOT_FOUND,
    error_code::PROTOCOL_INCOMPATIBLE,
    error_code::RECREATE_REQUIRED,
    error_code::UNAUTHORIZED,
    error_code::UNSUPPORTED_METHOD,
    error_code::WORKER_START_FAILED,
    error_code::WORKER_UNREACHABLE,
    error_code::WORKER_SHUTTING_DOWN,
    lease_code::HELD_BY_OTHER,
    lease_code::STALE_GENERATION,
    lease_code::LEASE_EXPIRED,
    lease_code::NOT_HOLDER,
    lease_code::CONTROLLER_INSTANCE_MISMATCH,
    lease_code::SESSION_NOT_RUNNING,
];
