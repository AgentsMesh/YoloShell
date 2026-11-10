use super::*;
use crate::model::PROTOCOL_VERSION;
use crate::protocol::ProtocolError;
use std::path::PathBuf;

const EXPECTED_HOME: &str = "/tmp/yolo-expected";
const REPORTED_HOME: &str = "/tmp/yolo-reported";
const SOCKET_PATH: &str = "/tmp/yolo-host.sock";
const IO_CONTEXT: &str = "connection refused";
const TEST_HOST_PID: u32 = 17;
const TEST_CONTROLLER_GENERATION: u64 = 19;
const INCOMPATIBLE_VERSION_OFFSET: u16 = 1;

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn startup_poll_retries_only_transport_errors() {
    assert!(poll_host_status(Ok(status())).unwrap());
    let transport = ClientError::Protocol(ProtocolError::Io(std::io::Error::new(
        std::io::ErrorKind::ConnectionRefused,
        IO_CONTEXT,
    )));
    assert!(!poll_host_status(Err(transport)).unwrap());
    assert!(!poll_host_status(Err(ClientError::Protocol(ProtocolError::OutcomeUnknown))).unwrap());
    for error in identity_errors() {
        assert!(poll_host_status(Err(error)).is_err());
    }
}

fn identity_errors() -> [ClientError; 3] {
    [
        ClientError::HostHomeMismatch {
            socket: SOCKET_PATH.into(),
            expected: EXPECTED_HOME.into(),
            reported: REPORTED_HOME.into(),
        },
        ClientError::InvalidHostHome {
            home: PathBuf::from(REPORTED_HOME),
            message: IO_CONTEXT.into(),
        },
        ClientError::HostProtocolMismatch {
            expected: PROTOCOL_VERSION,
            reported: PROTOCOL_VERSION.saturating_add(INCOMPATIBLE_VERSION_OFFSET),
        },
    ]
}

fn status() -> HostStatus {
    HostStatus {
        protocol_version: PROTOCOL_VERSION,
        boot_id: String::new(),
        host_pid: TEST_HOST_PID,
        controller_generation: TEST_CONTROLLER_GENERATION,
        home: EXPECTED_HOME.into(),
        capabilities: Vec::new(),
    }
}
