use crate::native_diagnostics::RpcResult;
use yolo_client::ClientError;
use yolo_runtime::protocol::{PROTOCOL_VERSION, ProtocolError};

pub const PRIVATE_MARKER: &str = "/private/native-diagnostic-secret-canary";
const VERSION_STEP: u16 = 1;

pub fn remote(code: &str) -> ClientError {
    ClientError::Remote {
        code: code.into(),
        message: PRIVATE_MARKER.into(),
        retryable: true,
        details: Some(serde_json::Value::String(PRIVATE_MARKER.into())),
    }
}

pub fn failures() -> Vec<(ClientError, RpcResult)> {
    vec![
        (
            ClientError::Protocol(ProtocolError::Io(std::io::Error::other(PRIVATE_MARKER))),
            RpcResult::Protocol,
        ),
        (
            ClientError::InvalidLocalHome {
                home: PRIVATE_MARKER.into(),
                message: PRIVATE_MARKER.into(),
            },
            RpcResult::InvalidLocalHome,
        ),
        (
            ClientError::InvalidHostHome {
                home: PRIVATE_MARKER.into(),
                message: PRIVATE_MARKER.into(),
            },
            RpcResult::InvalidHostHome,
        ),
        (
            ClientError::HostProtocolMismatch {
                expected: PROTOCOL_VERSION,
                reported: PROTOCOL_VERSION + VERSION_STEP,
            },
            RpcResult::HostProtocolMismatch,
        ),
        (
            ClientError::HostCapabilityUnavailable {
                capability: PRIVATE_MARKER.into(),
            },
            RpcResult::HostCapabilityUnavailable,
        ),
        (
            ClientError::HostHomeMismatch {
                socket: PRIVATE_MARKER.into(),
                expected: PRIVATE_MARKER.into(),
                reported: PRIVATE_MARKER.into(),
            },
            RpcResult::HostHomeMismatch,
        ),
        (
            ClientError::RuntimeUnavailable {
                terminal_id: uuid::Uuid::new_v4(),
            },
            RpcResult::RuntimeUnavailable,
        ),
        (
            ClientError::IncompleteCatalogInventory,
            RpcResult::IncompleteCatalogInventory,
        ),
    ]
}
