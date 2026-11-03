use crate::{FIRST_REVISION, LayoutAddress, SharedTab, ProtocolError, identity, validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Shared topology and terminal bindings. Focus, scrolling and writer leases are client-local.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "DocumentFields")]
pub struct SharedLayoutDocument {
    pub address: LayoutAddress,
    pub revision: u64,
    #[serde(with = "crate::identity")]
    pub last_request_id: Uuid,
    pub tabs: Vec<SharedTab>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentFields {
    address: LayoutAddress,
    revision: u64,
    #[serde(with = "crate::identity")]
    last_request_id: Uuid,
    tabs: Vec<SharedTab>,
}

impl SharedLayoutDocument {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        identity::require(self.address.layout_id)?;
        identity::require(self.last_request_id)?;
        if self.revision < FIRST_REVISION {
            return Err(ProtocolError::InvalidRevision);
        }
        validation::tabs(&self.tabs)
    }
}
impl TryFrom<DocumentFields> for SharedLayoutDocument {
    type Error = ProtocolError;
    fn try_from(value: DocumentFields) -> Result<Self, Self::Error> {
        let document = Self {
            address: value.address,
            revision: value.revision,
            last_request_id: value.last_request_id,
            tabs: value.tabs,
        };
        document.validate()?;
        Ok(document)
    }
}
