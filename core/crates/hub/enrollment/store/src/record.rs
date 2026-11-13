use crate::{EnrollmentRecord, constants::*};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use yolo_paths::{directory::PrivateDirectory, identity::DurableObjectIdentity};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StoredRecord {
    pub schema_version: u16,
    pub parent: DurableObjectIdentity,
    pub root: DurableObjectIdentity,
    pub entry: DurableObjectIdentity,
    pub record: EnrollmentRecord,
}
impl StoredRecord {
    pub fn validate(
        &self,
        parent: &PrivateDirectory,
        root: &PrivateDirectory,
        entry: &PrivateDirectory,
    ) -> Result<()> {
        ensure!(
            self.schema_version == RECORD_SCHEMA
                && self.parent == parent.durable_identity()?
                && self.root == root.durable_identity()?
                && self.entry == entry.durable_identity()?,
            CHANGED
        );
        self.record.validate()
    }
}
