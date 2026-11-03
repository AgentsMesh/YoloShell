use crate::client::ClientError;
use crate::model::{normalize_terminal_columns, normalize_terminal_rows};
use crate::protocol::ProtocolError;
use std::io;
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

#[path = "cancellation.rs"]
mod cancellation;
#[path = "constants.rs"]
mod constants;
#[path = "control.rs"]
mod control;
#[path = "host.rs"]
mod host;
#[path = "lease.rs"]
mod lease;
#[path = "output.rs"]
mod output;
#[path = "session.rs"]
mod session;
#[path = "status.rs"]
mod status;
#[path = "tty.rs"]
mod tty;

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/session_support.rs"]
mod session_test_support;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/support.rs"]
mod test_support;

pub use cancellation::RelayCancellation;
pub use host::{RelayRuntime, runtime_executable_path};
pub use session::{
    relay_session, relay_session_with_status, relay_session_with_status_and_cancellation,
};
pub use tty::{RawTerminal, terminal_size, watch_window_changes};

pub const INITIAL_OUTPUT_SEQUENCE: u64 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TerminalSize {
    pub cols: u16,
    pub rows: u16,
}

impl TerminalSize {
    pub fn new(cols: u16, rows: u16) -> Option<Self> {
        (cols != 0 && rows != 0).then_some(Self {
            cols: normalize_terminal_columns(cols),
            rows: normalize_terminal_rows(rows),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelayOptions {
    pub terminal_id: Uuid,
    pub holder_id: String,
    pub take_control: bool,
    pub after_sequence: u64,
    pub initial_size: Option<TerminalSize>,
}

impl RelayOptions {
    pub fn new(terminal_id: Uuid, holder_id: impl Into<String>) -> Result<Self, RelayError> {
        let holder_id = holder_id.into();
        if holder_id.trim().is_empty() {
            return Err(RelayError::InvalidHolderId);
        }
        Ok(Self {
            terminal_id,
            holder_id,
            take_control: false,
            after_sequence: INITIAL_OUTPUT_SEQUENCE,
            initial_size: None,
        })
    }

    pub fn after_sequence(mut self, sequence: u64) -> Self {
        self.after_sequence = sequence;
        self
    }

    pub fn initial_size(mut self, size: Option<TerminalSize>) -> Self {
        self.initial_size = size;
        self
    }

    pub fn take_control(mut self, take_control: bool) -> Self {
        self.take_control = take_control;
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelayExit {
    InputClosed,
    ReadOnlyInputClosed,
    SessionExited,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RelayStatus {
    ControlAcquired,
    ReadOnly { holder_id: Option<String> },
    ControlLost { holder_id: Option<String> },
    HostStarted { executable: PathBuf },
    SessionResumed,
    SessionEnding,
    SessionExited,
}

#[derive(Debug, Error)]
pub enum RelayError {
    #[error("writer holder id must not be empty")]
    InvalidHolderId,
    #[error("session {0} is archived and cannot be attached")]
    ArchivedSession(Uuid),
    #[error("session {0} did not become attachable after recovery")]
    SessionNotAttachable(Uuid),
    #[error("writer takeover did not converge because ownership kept changing")]
    TakeoverContended,
    #[error("host returned an invalid writer lease: {0}")]
    InvalidLease(&'static str),
    #[error("runtime executable path must be absolute and name a file: {0}")]
    InvalidRuntimeExecutable(PathBuf),
    #[error("yolo-host did not become ready at {socket}")]
    HostStartupTimeout { socket: PathBuf },
    #[error("failed to start yolo-host at {executable}: {source}")]
    HostSpawn {
        executable: PathBuf,
        source: anyhow::Error,
    },
    #[error("attachment event targets {actual:?}, expected session {expected}")]
    UnexpectedSession {
        actual: Option<String>,
        expected: Uuid,
    },
    #[error("attachment runtime changed from {expected} to {actual}")]
    RuntimeEpochChanged { expected: Uuid, actual: Uuid },
    #[error("terminal relay was cancelled")]
    Cancelled,
    #[error(transparent)]
    Client(#[from] ClientError),
    #[error(transparent)]
    ObservationInterrupted(ClientError),
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
    #[error(transparent)]
    Io(#[from] io::Error),
}

pub(crate) fn client_error_is_retryable(error: &ClientError) -> bool {
    !lease::has_session_not_running_code(error)
        && (host::is_transport_error(error)
            || matches!(
                error,
                ClientError::Remote {
                    retryable: true,
                    ..
                }
            ))
}

pub(crate) fn relay_error_is_retryable(error: &RelayError) -> bool {
    matches!(error, RelayError::ObservationInterrupted(_))
        || matches!(error, RelayError::Client(error) if client_error_is_retryable(error))
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/tests.rs"]
mod tests;
