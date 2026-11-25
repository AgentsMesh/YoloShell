use yolo_hub_client::ClientError;
use yolo_hub_terminal_protocol::PublicError;

#[derive(
    Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, thiserror::Error,
)]
#[serde(rename_all = "snake_case")]
pub enum ManagementError {
    #[error("Hub configuration is unavailable")]
    Unavailable,
    #[error("Hub configuration management is unsupported")]
    Unsupported,
    #[error("Hub configuration changed; reload is required")]
    Conflict,
    #[error("Hub configuration change outcome is unknown")]
    OutcomeUnknown,
    #[error("Invalid Hub configuration request")]
    InvalidRequest,
    #[error("Hub configuration request was rejected")]
    Rejected,
}

impl From<ClientError> for ManagementError {
    fn from(error: ClientError) -> Self {
        if error.outcome_is_unknown() {
            return Self::OutcomeUnknown;
        }
        match error {
            ClientError::Unsupported | ClientError::Remote(PublicError::Unsupported) => {
                Self::Unsupported
            }
            ClientError::Remote(PublicError::Conflict) => Self::Conflict,
            ClientError::Validation(_) => Self::InvalidRequest,
            ClientError::Remote(PublicError::Unreachable) => Self::Unavailable,
            ClientError::Remote(_) | ClientError::WrongLocalUser => Self::Rejected,
            _ => Self::Unavailable,
        }
    }
}
