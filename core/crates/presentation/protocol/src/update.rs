use crate::{
    FIRST_REVISION, LayoutAddress, SharedLayoutDocument, SharedTab, ProtocolError, identity, validation,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Compare-and-swap the whole document. Zero expected revision creates an absent document.
/// A lost response is resolved with Get; this request never authorizes input or termination.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "UpdateFields")]
pub struct LayoutUpdate {
    pub address: LayoutAddress,
    #[serde(with = "crate::identity")]
    pub request_id: Uuid,
    pub expected_revision: u64,
    pub tabs: Vec<SharedTab>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateFields {
    address: LayoutAddress,
    #[serde(with = "crate::identity")]
    request_id: Uuid,
    expected_revision: u64,
    tabs: Vec<SharedTab>,
}

impl LayoutUpdate {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        identity::require(self.address.layout_id)?;
        identity::require(self.request_id)?;
        self.expected_revision
            .checked_add(FIRST_REVISION)
            .ok_or(ProtocolError::InvalidRevision)?;
        validation::tabs(&self.tabs)
    }
    pub fn apply(&self, current: Option<&SharedLayoutDocument>) -> Result<SharedLayoutDocument, ProtocolError> {
        self.validate()?;
        if let Some(document) = current {
            document.validate()?;
        }
        if current.is_some_and(|document| document.address != self.address) {
            return Err(ProtocolError::AddressMismatch);
        }
        if current.map_or(0, |document| document.revision) != self.expected_revision {
            return Err(ProtocolError::RevisionConflict);
        }
        Ok(SharedLayoutDocument {
            address: self.address,
            revision: self.expected_revision + FIRST_REVISION,
            last_request_id: self.request_id,
            tabs: self.tabs.clone(),
        })
    }
}
impl TryFrom<UpdateFields> for LayoutUpdate {
    type Error = ProtocolError;
    fn try_from(value: UpdateFields) -> Result<Self, Self::Error> {
        let update = Self {
            address: value.address,
            request_id: value.request_id,
            expected_revision: value.expected_revision,
            tabs: value.tabs,
        };
        update.validate()?;
        Ok(update)
    }
}
