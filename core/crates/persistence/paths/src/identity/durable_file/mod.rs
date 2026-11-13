use super::{DurableObjectIdentity, FileIdentity, FileTimestamp};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

mod capture;

const VERSION_SCHEMA: u16 = 1;
const INVALID_VERSION: &str = "durable file version is invalid";
const NANOSECONDS_PER_SECOND: i64 = 1_000_000_000;

/// Persisted object and content-version evidence. It is not a live CAS token.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Record", into = "Record")]
pub struct DurableFileVersion {
    record: Record,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema: u16,
    object: DurableObjectIdentity,
    length: u64,
    modified: FileTimestamp,
    changed: FileTimestamp,
}

impl DurableFileVersion {
    pub fn length(self) -> u64 {
        self.record.length
    }
    pub fn object_identity(self) -> DurableObjectIdentity {
        self.record.object
    }
    pub fn same_object(self, other: Self) -> bool {
        self.record.object == other.record.object
    }
    pub fn same_content_version(self, other: Self) -> bool {
        self.same_object(other)
            && self.record.length == other.record.length
            && self.record.modified == other.record.modified
    }
    pub fn unchanged(self, other: Self) -> bool {
        self.same_content_version(other) && self.record.changed == other.record.changed
    }
    pub fn content_version_evidence(self) -> (DurableObjectIdentity, u64, FileTimestamp) {
        (self.record.object, self.record.length, self.record.modified)
    }
    fn from_live(object: DurableObjectIdentity, live: FileIdentity) -> Result<Self> {
        Record {
            schema: VERSION_SCHEMA,
            object,
            length: live.length,
            modified: live.modified,
            changed: live.changed,
        }
        .try_into()
    }
}

impl TryFrom<Record> for DurableFileVersion {
    type Error = anyhow::Error;
    fn try_from(record: Record) -> Result<Self> {
        ensure!(
            record.schema == VERSION_SCHEMA
                && record.object.kind() == yolo_durable_identity::ObjectKind::RegularFile
                && [record.modified, record.changed]
                    .iter()
                    .all(
                        |time| (i64::default()..NANOSECONDS_PER_SECOND).contains(&time.nanoseconds)
                    ),
            INVALID_VERSION
        );
        Ok(Self { record })
    }
}

impl From<DurableFileVersion> for Record {
    fn from(value: DurableFileVersion) -> Self {
        value.record
    }
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/durable/durable_file_version.rs"]
mod integration_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../../unit_tests/durable/durable_file_version.rs"]
mod unit_tests;
