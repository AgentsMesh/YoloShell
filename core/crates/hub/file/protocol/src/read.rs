use crate::{FileRootTarget, RelativeFilePath, constants::*};
use serde::{Deserialize, Serialize};
use yolo_crypto::Sha256Digest;
use yolo_hub_terminal_protocol::PublicError;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadFileFields {
    pub schema_version: u16,
    pub target: FileRootTarget,
    pub relative_path: RelativeFilePath,
    pub offset: u64,
    pub length: u32,
    /// Continuations bind the origin-issued version of the same target and path.
    pub expected_version: Option<Sha256Digest>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ReadFileFields", into = "ReadFileFields")]
pub struct ReadFileRequest(ReadFileFields);

impl ReadFileRequest {
    pub fn new(fields: ReadFileFields) -> Result<Self, PublicError> {
        if fields.schema_version != FILE_PROTOCOL_SCHEMA_VERSION {
            return Err(PublicError::UnsupportedSchema);
        }
        if fields.length == 0
            || fields.length > MAX_FILE_READ_BYTES
            || fields
                .offset
                .checked_add(u64::from(fields.length))
                .is_none_or(|end| end > MAX_FILE_OFFSET)
            || (fields.offset != 0 && fields.expected_version.is_none())
        {
            return Err(PublicError::InvalidRequest);
        }
        Ok(Self(fields))
    }
    pub fn fields(&self) -> &ReadFileFields {
        &self.0
    }
}
impl TryFrom<ReadFileFields> for ReadFileRequest {
    type Error = PublicError;
    fn try_from(value: ReadFileFields) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<ReadFileRequest> for ReadFileFields {
    fn from(value: ReadFileRequest) -> Self {
        value.0
    }
}
