use crate::constants::*;
use std::{error::Error, fmt};
use yolo_hub_client::ClientError;

pub enum RelayError {
    Client(ClientError),
    Io,
    StatusClosed,
    CommandClosed,
    OutputDeadline,
    Unreachable,
    ResyncRequired,
    InvalidReply,
    InvalidOptions,
}
impl fmt::Display for RelayError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Client(error) if error.outcome_is_unknown() => {
                formatter.write_str(UNKNOWN_OUTCOME)
            }
            Self::Client(_) => formatter.write_str(CLIENT_FAILURE),
            Self::Io => formatter.write_str(IO_FAILURE),
            Self::StatusClosed => formatter.write_str(STATUS_FAILURE),
            Self::CommandClosed => formatter.write_str(COMMAND_CLOSED),
            Self::OutputDeadline => formatter.write_str(OUTPUT_FAILURE),
            Self::Unreachable => formatter.write_str(UNREACHABLE),
            Self::ResyncRequired => formatter.write_str(RESYNC_REQUIRED),
            Self::InvalidReply => formatter.write_str(INVALID_REPLY),
            Self::InvalidOptions => formatter.write_str(INVALID_OPTIONS),
        }
    }
}
impl fmt::Debug for RelayError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}
impl Error for RelayError {}
impl From<ClientError> for RelayError {
    fn from(error: ClientError) -> Self {
        Self::Client(error)
    }
}
impl From<std::io::Error> for RelayError {
    fn from(_: std::io::Error) -> Self {
        Self::Io
    }
}
