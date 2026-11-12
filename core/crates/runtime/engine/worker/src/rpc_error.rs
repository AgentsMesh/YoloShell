use super::*;

#[derive(Debug, thiserror::Error)]
pub enum WorkerRpcError {
    #[error("worker stream ended")]
    Closed,
    #[error("worker transport error: {0}")]
    Transport(String),
    #[error("worker rejected request: {0:?}")]
    Remote(ErrorBody),
    #[error("worker protocol error: {0}")]
    Protocol(String),
    #[error("worker identity mismatch: {0}")]
    IdentityMismatch(String),
    #[error("worker request outcome is unknown; do not replay the request")]
    OutcomeUnknown,
}

impl WorkerRpcError {
    pub fn from_rpc(error: yolo_capnp_rpc::Error) -> Self {
        match error {
            yolo_capnp_rpc::Error::Closed => Self::Closed,
            yolo_capnp_rpc::Error::OutcomeUnknown => Self::OutcomeUnknown,
            other => Self::transport(other),
        }
    }
    pub fn transport(error: impl std::fmt::Display) -> Self {
        Self::Transport(error.to_string())
    }

    pub fn remote(error: ErrorBody) -> Self {
        Self::Remote(error)
    }

    pub fn protocol(error: impl std::fmt::Display) -> Self {
        Self::Protocol(error.to_string())
    }

    pub fn identity_mismatch(error: impl std::fmt::Display) -> Self {
        Self::IdentityMismatch(error.to_string())
    }

    pub fn from_protocol(error: ProtocolError) -> Self {
        match error {
            ProtocolError::OutcomeUnknown => Self::OutcomeUnknown,
            ProtocolError::Io(error) => Self::transport(error),
            ProtocolError::EndOfStream => Self::Closed,
            ProtocolError::Remote(error) => Self::remote(error),
            error => Self::protocol(error),
        }
    }

    pub fn is_transport(&self) -> bool {
        matches!(self, Self::Closed | Self::Transport(_))
    }

    pub fn into_error_body(self) -> ErrorBody {
        match self {
            Self::Closed => ErrorBody::new(
                crate::protocol::error_code::WORKER_UNREACHABLE,
                ProtocolError::EndOfStream.to_string(),
                true,
            ),
            Self::OutcomeUnknown => ErrorBody::new(
                crate::protocol::error_code::DELIVERY_UNKNOWN,
                ProtocolError::OutcomeUnknown.to_string(),
                false,
            ),
            Self::Transport(detail) => ErrorBody::new(
                crate::protocol::error_code::WORKER_UNREACHABLE,
                detail,
                true,
            ),
            Self::Remote(error) => error,
            Self::Protocol(detail) => ErrorBody::protocol_incompatible(detail),
            Self::IdentityMismatch(detail) => ErrorBody::new(
                crate::protocol::error_code::IDENTITY_MISMATCH,
                detail,
                false,
            ),
        }
    }
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../unit_tests/rpc/rpc_outcome.rs"]
mod rpc_outcome_tests;

pub(super) async fn worker_rpc_with_timeout<F, T>(
    timeout: Duration,
    context: &'static str,
    operation: F,
) -> std::result::Result<T, WorkerRpcError>
where
    F: Future<Output = std::result::Result<T, WorkerRpcError>>,
{
    tokio::time::timeout(timeout, operation)
        .await
        .map_err(|_| WorkerRpcError::transport(context))?
}
