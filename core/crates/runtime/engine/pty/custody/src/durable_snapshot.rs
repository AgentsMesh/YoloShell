use super::CustodyDurableRecord;
use crate::paths::identity::FileIdentity;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustodyDurableSnapshot {
    record: CustodyDurableRecord,
    file_identity: FileIdentity,
}

impl CustodyDurableSnapshot {
    pub fn new(record: CustodyDurableRecord, file_identity: FileIdentity) -> Self {
        Self {
            record,
            file_identity,
        }
    }

    pub fn record(&self) -> &CustodyDurableRecord {
        &self.record
    }

    pub fn file_identity(&self) -> FileIdentity {
        self.file_identity
    }

    pub fn into_record(self) -> CustodyDurableRecord {
        self.record
    }
}
