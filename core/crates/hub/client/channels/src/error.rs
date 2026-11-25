use crate::constants::{
    INVALIDATED_REQUEST, NOT_CONNECTED, OPERATION_LIMIT, QUERY_LIMIT, QUERY_TASK_FAILED,
    STALE_GENERATION,
};
use std::{error::Error, fmt};
use yolo_hub_client::ClientError;

#[derive(Debug)]
pub enum ControlError {
    NotConnected,
    StaleGeneration,
    Invalidated,
    OperationLimit,
    QueryLimit,
    QueryTaskFailed,
    Client(ClientError),
}

impl fmt::Display for ControlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConnected => formatter.write_str(NOT_CONNECTED),
            Self::StaleGeneration => formatter.write_str(STALE_GENERATION),
            Self::Invalidated => formatter.write_str(INVALIDATED_REQUEST),
            Self::OperationLimit => formatter.write_str(OPERATION_LIMIT),
            Self::QueryLimit => formatter.write_str(QUERY_LIMIT),
            Self::QueryTaskFailed => formatter.write_str(QUERY_TASK_FAILED),
            Self::Client(error) => fmt::Display::fmt(error, formatter),
        }
    }
}

impl Error for ControlError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Client(error) => Some(error),
            Self::NotConnected
            | Self::StaleGeneration
            | Self::Invalidated
            | Self::OperationLimit
            | Self::QueryLimit
            | Self::QueryTaskFailed => None,
        }
    }
}

impl From<ClientError> for ControlError {
    fn from(value: ClientError) -> Self {
        Self::Client(value)
    }
}
