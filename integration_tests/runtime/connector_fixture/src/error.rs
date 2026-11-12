use crate::constants::*;
use std::{error::Error, fmt};
use yolo_hub_connector_client::ClientError;

pub enum FixtureError {
    InvalidRequest,
    Input,
    Output,
    TooLarge,
    Deadline,
    Client(ClientError),
}

impl fmt::Display for FixtureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::InvalidRequest => INVALID_INPUT,
            Self::Input => INPUT_FAILED,
            Self::Output => OUTPUT_FAILED,
            Self::TooLarge => LINE_TOO_LARGE,
            Self::Deadline => DEADLINE,
            Self::Client(error) => return fmt::Display::fmt(error, formatter),
        };
        formatter.write_str(label)
    }
}
impl fmt::Debug for FixtureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}
impl Error for FixtureError {}
impl From<ClientError> for FixtureError {
    fn from(error: ClientError) -> Self {
        Self::Client(error)
    }
}
