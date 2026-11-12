use crate::constants::{CHANNEL_LIMIT, CLOSED};
use std::{error::Error, fmt};
use tokio::sync::TryAcquireError;

/// Local channel admission is distinct from a failed or rejected SSH channel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChannelAdmissionError {
    AtCapacity,
    Closed,
}

impl From<TryAcquireError> for ChannelAdmissionError {
    fn from(error: TryAcquireError) -> Self {
        match error {
            TryAcquireError::NoPermits => Self::AtCapacity,
            TryAcquireError::Closed => Self::Closed,
        }
    }
}

impl fmt::Display for ChannelAdmissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::AtCapacity => CHANNEL_LIMIT,
            Self::Closed => CLOSED,
        })
    }
}
impl Error for ChannelAdmissionError {}
