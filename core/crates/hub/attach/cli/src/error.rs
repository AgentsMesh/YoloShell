use crate::constants::*;
use std::{error::Error, fmt};
use yolo_hub_terminal_relay::RelayError;

pub enum CliError {
    Io,
    Status,
    StatusDescriptor,
    Command,
    CommandDescriptor,
    Relay(RelayError),
}
impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io => f.write_str(IO_FAILURE),
            Self::Status => f.write_str(STATUS_FAILURE),
            Self::StatusDescriptor => f.write_str(STATUS_DESCRIPTOR_INVALID),
            Self::Command => f.write_str(COMMAND_FAILURE),
            Self::CommandDescriptor => f.write_str(COMMAND_DESCRIPTOR_INVALID),
            Self::Relay(error) => fmt::Display::fmt(error, f),
        }
    }
}
impl fmt::Debug for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for CliError {}
impl From<std::io::Error> for CliError {
    fn from(_: std::io::Error) -> Self {
        Self::Io
    }
}
impl From<RelayError> for CliError {
    fn from(error: RelayError) -> Self {
        Self::Relay(error)
    }
}
