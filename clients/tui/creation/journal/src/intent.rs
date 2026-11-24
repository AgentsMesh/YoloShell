use crate::constants::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt;
use yolo_hub_identity::HubId;
use yolo_hub_terminal_protocol::CreateShellRequest;

/// Persist before the first Create delivery. Retained evidence is not permission
/// to send Create again: resolve an existing intent through authenticated queries.
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreationIntent {
    pub origin: HubId,
    pub request: CreateShellRequest,
}

impl CreationIntent {
    pub fn new(origin: HubId, request: CreateShellRequest) -> Result<Self> {
        let intent = Self { origin, request };
        intent.validate()?;
        Ok(intent)
    }

    pub(crate) fn validate(&self) -> Result<()> {
        CreateShellRequest::new(self.request.fields().clone())?;
        Ok(())
    }

    pub(crate) fn overlaps_request(&self, request: &CreateShellRequest) -> bool {
        self.request.fields().request_id == request.fields().request_id
            || self.request.fields().creation_token == request.fields().creation_token
    }
}

impl fmt::Debug for CreationIntent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct(DEBUG_INTENT)
            .field(DEBUG_ORIGIN, &self.origin)
            .field(DEBUG_REQUEST_ID, &self.request.fields().request_id)
            .finish_non_exhaustive()
    }
}
