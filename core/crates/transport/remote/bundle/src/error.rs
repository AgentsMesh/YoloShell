use crate::constants::UNCERTAIN;
use std::fmt;
use yolo_distribution_publication_protocol::PublicationRequest;

#[derive(Debug)]
pub struct UncertainPublication {
    pub(crate) request: PublicationRequest,
    pub(crate) cause: anyhow::Error,
}

impl UncertainPublication {
    pub fn request(&self) -> &PublicationRequest {
        &self.request
    }
}

impl fmt::Display for UncertainPublication {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{UNCERTAIN}: {}", self.cause)
    }
}
impl std::error::Error for UncertainPublication {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.cause.as_ref())
    }
}
