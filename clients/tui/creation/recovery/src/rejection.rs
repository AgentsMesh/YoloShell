use crate::constants::*;
use std::fmt;
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::ControlError;
use yolo_hub_terminal_protocol::PublicError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockedReason {
    Unknown,
    Rejected(PublicError),
    InvalidResponse,
    Unsupported,
    InvalidEndpoint,
    WrongLocalUser,
}

impl fmt::Display for BlockedReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown => formatter.write_str(UNKNOWN),
            Self::Rejected(code) => write!(formatter, "{REJECTED}: {code}"),
            Self::InvalidResponse => formatter.write_str(INVALID_RESPONSE),
            Self::Unsupported => formatter.write_str(UNSUPPORTED),
            Self::InvalidEndpoint => formatter.write_str(INVALID_ENDPOINT),
            Self::WrongLocalUser => formatter.write_str(WRONG_LOCAL_USER),
        }
    }
}

pub(crate) fn remote(code: PublicError) -> Option<BlockedReason> {
    match code {
        PublicError::Unreachable | PublicError::Internal => None,
        PublicError::OutcomeUnknown => Some(BlockedReason::Unknown),
        _ => Some(BlockedReason::Rejected(code)),
    }
}

pub(crate) fn error(error: &ControlError) -> Option<BlockedReason> {
    match error {
        ControlError::NotConnected
        | ControlError::StaleGeneration
        | ControlError::Invalidated
        | ControlError::OperationLimit
        | ControlError::QueryLimit
        | ControlError::QueryTaskFailed => None,
        ControlError::Client(error) => match error {
            ClientError::Transport(_) | ClientError::OutcomeUnknown(_) | ClientError::Poisoned => {
                None
            }
            ClientError::Remote(code) => remote(*code),
            ClientError::Validation(code) => Some(BlockedReason::Rejected(*code)),
            ClientError::InvalidResponse(_)
            | ClientError::OutcomeUnknownResponse(_)
            | ClientError::TerminalStream => Some(BlockedReason::InvalidResponse),
            ClientError::Unsupported | ClientError::UnsupportedEndpoint => {
                Some(BlockedReason::Unsupported)
            }
            ClientError::InvalidEndpoint => Some(BlockedReason::InvalidEndpoint),
            ClientError::WrongLocalUser => Some(BlockedReason::WrongLocalUser),
        },
    }
}
