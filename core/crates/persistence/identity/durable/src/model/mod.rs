use crate::Error;
use serde::{Deserialize, Serialize};

mod handle;
#[cfg(all(test, feature = "yolo-unit-tests"))]
pub(crate) use handle::Bytes;
pub(crate) use handle::Handle;
pub(crate) const UUID_BYTES: usize = 16;
pub(crate) const HANDLE_CAPACITY: usize = 128;
const SCHEMA: u16 = 1;
const NANOSECONDS_PER_SECOND: u32 = 1_000_000_000;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Record", into = "Record")]
pub struct DurableObjectIdentity {
    pub(crate) record: Record,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectKind {
    Directory,
    RegularFile,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Birth {
    pub seconds: i64,
    pub nanoseconds: u32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Proof {
    Linux {
        filesystem_type: u32,
        filesystem_uuid: [u8; UUID_BYTES],
        handle: Handle,
    },
    Macos {
        volume_uuid: [u8; UUID_BYTES],
        file_id: u64,
    },
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Record {
    pub schema: u16,
    pub kind: ObjectKind,
    pub birth: Birth,
    pub proof: Proof,
}

impl DurableObjectIdentity {
    pub(crate) fn new(kind: ObjectKind, birth: Birth, proof: Proof) -> Result<Self, Error> {
        Record {
            schema: SCHEMA,
            kind,
            birth,
            proof,
        }
        .try_into()
    }
}

impl TryFrom<Record> for DurableObjectIdentity {
    type Error = Error;
    fn try_from(record: Record) -> Result<Self, Error> {
        if record.schema != SCHEMA {
            return Err(Error::Invalid("schema"));
        }
        if record.birth.nanoseconds >= NANOSECONDS_PER_SECOND {
            return Err(Error::Invalid("birth nanoseconds"));
        }
        let uuid = match record.proof {
            Proof::Linux {
                filesystem_type,
                filesystem_uuid,
                handle,
            } => {
                if !supported_linux_filesystem(filesystem_type) {
                    return Err(Error::Invalid("Linux filesystem type"));
                }
                handle.validate()?;
                filesystem_uuid
            }
            Proof::Macos {
                volume_uuid,
                file_id,
            } => {
                if file_id == u64::default() {
                    return Err(Error::Invalid("Mac file ID"));
                }
                volume_uuid
            }
        };
        if uuid.iter().all(|byte| *byte == u8::default()) {
            return Err(Error::Invalid("filesystem UUID"));
        }
        Ok(Self { record })
    }
}

impl From<DurableObjectIdentity> for Record {
    fn from(identity: DurableObjectIdentity) -> Self {
        identity.record
    }
}

pub(crate) const BTRFS_MAGIC: u32 = 0x9123_683e;
pub(crate) const EXT4_MAGIC: u32 = 0xef53;
pub(crate) const XFS_MAGIC: u32 = 0x5846_5342;
pub(crate) const TMPFS_MAGIC: u32 = 0x0102_1994;

pub(crate) fn supported_linux_filesystem(value: u32) -> bool {
    matches!(value, BTRFS_MAGIC | EXT4_MAGIC | XFS_MAGIC | TMPFS_MAGIC)
}
