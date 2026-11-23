use crate::paths::identity::FileIdentity;
use serde::{Deserialize, Serialize};

pub(super) const CUSTODY_RETIREMENT_EVIDENCE_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustodyRetirementEvidence {
    schema_version: u16,
    controller_generation: u64,
    runtime_file_identity: FileIdentity,
    journal_file_identity: FileIdentity,
    journal_cursor_file_identity: FileIdentity,
    last_output_sequence: u64,
}

impl CustodyRetirementEvidence {
    pub fn new(
        controller_generation: u64,
        runtime_file_identity: FileIdentity,
        journal_file_identity: FileIdentity,
        journal_cursor_file_identity: FileIdentity,
        last_output_sequence: u64,
    ) -> Self {
        Self {
            schema_version: CUSTODY_RETIREMENT_EVIDENCE_SCHEMA_VERSION,
            controller_generation,
            runtime_file_identity,
            journal_file_identity,
            journal_cursor_file_identity,
            last_output_sequence,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.schema_version == CUSTODY_RETIREMENT_EVIDENCE_SCHEMA_VERSION
            && self.controller_generation != u64::default()
    }

    pub fn controller_generation(self) -> u64 {
        self.controller_generation
    }

    pub fn runtime_file_identity(self) -> FileIdentity {
        self.runtime_file_identity
    }
}
