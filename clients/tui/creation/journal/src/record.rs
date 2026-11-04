use crate::CreationIntent;
use crate::constants::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::fmt;
use yolo_hub_identity::HubId;
use yolo_hub_terminal_protocol::{CreateShellRequest, CreateShellResult};
use yolo_paths::identity::DurableObjectIdentity;

/// Evidence for confirming an already-created shell. This is never writer authority.
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PendingFinalization {
    pub origin: HubId,
    pub request: CreateShellRequest,
    pub creation: CreateShellResult,
}

impl PendingFinalization {
    pub fn new(
        origin: HubId,
        request: CreateShellRequest,
        creation: CreateShellResult,
    ) -> Result<Self> {
        let record = Self {
            origin,
            request,
            creation,
        };
        record.validate()?;
        Ok(record)
    }

    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(self.creation.address.hub_id == self.origin, INVALID_RECORD);
        Ok(())
    }

    pub(crate) fn overlaps(&self, other: &Self) -> bool {
        self.request.fields().request_id == other.request.fields().request_id
            || self.request.fields().creation_token == other.request.fields().creation_token
            || self.creation.address == other.creation.address
    }
}

impl fmt::Debug for PendingFinalization {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct(DEBUG_NAME)
            .field(DEBUG_ORIGIN, &self.origin)
            .field(DEBUG_REQUEST_ID, &self.request.fields().request_id)
            .field(DEBUG_ADDRESS, &self.creation.address)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Document {
    pub schema_version: u16,
    pub entry_hub: HubId,
    pub directory: DurableObjectIdentity,
    pub revision: u64,
    pub pending: Vec<PendingFinalization>,
    pub intents: Vec<CreationIntent>,
}

impl Document {
    pub fn validate(&self, entry: HubId, directory: DurableObjectIdentity) -> Result<()> {
        ensure!(
            self.schema_version == JOURNAL_SCHEMA_VERSION,
            INVALID_RECORD
        );
        ensure!(
            self.entry_hub == entry && self.directory == directory,
            WRONG_OWNER
        );
        ensure!(self.revision >= FIRST_REVISION, INVALID_RECORD);
        ensure!(self.pending.len() <= MAX_PENDING_CREATIONS, CAPACITY);
        ensure!(
            self.intents.len() <= MAX_PENDING_CREATIONS - self.pending.len(),
            CAPACITY
        );
        for (index, record) in self.pending.iter().enumerate() {
            record.validate()?;
            ensure!(
                !self.pending[..index]
                    .iter()
                    .any(|other| record.overlaps(other)),
                CONFLICT
            );
        }
        for (index, intent) in self.intents.iter().enumerate() {
            intent.validate()?;
            ensure!(
                !self.intents[..index]
                    .iter()
                    .any(|other| intent.overlaps_request(&other.request))
                    && !self
                        .pending
                        .iter()
                        .any(|record| intent.overlaps_request(&record.request)),
                CONFLICT
            );
        }
        Ok(())
    }
}
