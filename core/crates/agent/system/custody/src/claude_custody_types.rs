use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub(super) const LEDGER_SCHEMA_VERSION: u32 = 1;
pub(super) const GUARDIAN_COUNT: usize = 2;
pub(super) const MAX_RECORDS: usize = crate::claude_settings_artifact::MAX_ARTIFACTS;
pub(super) const MAX_LEDGER_BYTES: u64 = 256 * 1024;
#[cfg(test)]
pub(super) const LOCK_RETRY: std::time::Duration = std::time::Duration::from_millis(1);
#[cfg(test)]
pub(super) const LOCK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(8);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustodyLedger {
    pub(crate) schema_version: u32,
    pub(crate) records: Vec<CustodyRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CustodyRecord {
    #[serde(flatten)]
    pub(crate) spec: CustodySpec,
    pub(super) state: CustodyState,
    pub(super) publication: CustodyPublication,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustodySpec {
    #[serde(with = "crate::serde_uuid")]
    pub invocation_id: Uuid,
    pub target: DurableProcessIdentity,
    pub worker: DurableProcessIdentity,
    pub guardians: [DurableProcessIdentity; GUARDIAN_COUNT],
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum CustodyState {
    Armed,
    Containing,
    Settled,
    Indeterminate,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum CustodyPublication {
    Pending,
    Active,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DurableProcessIdentity {
    pub boot_id: String,
    pub pid: u32,
    pub start_time: u64,
}

impl DurableProcessIdentity {
    pub fn new(boot_id: String, identity: crate::process::ProcessIdentity) -> Self {
        Self {
            boot_id,
            pid: identity.pid,
            start_time: identity.start_time,
        }
    }

    pub fn process(&self) -> crate::process::ProcessIdentity {
        crate::process::ProcessIdentity {
            pid: self.pid,
            start_time: self.start_time,
        }
    }

    pub fn relation(&self) -> crate::process::ProcessIdentityRelation {
        match crate::process::boot_identity_relation(&self.boot_id, &crate::process::boot_id()) {
            crate::process::BootIdentityRelation::Different => {
                crate::process::ProcessIdentityRelation::Gone
            }
            crate::process::BootIdentityRelation::Same => {
                crate::process::process_identity_relation(self.pid, self.start_time)
            }
            crate::process::BootIdentityRelation::Unknown => {
                crate::process::ProcessIdentityRelation::Unknown
            }
        }
    }
}
