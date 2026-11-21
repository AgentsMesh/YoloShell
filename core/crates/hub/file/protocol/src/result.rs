use crate::{FileRootTarget, ReadFileRequest, RelativeFilePath, constants::*};
use serde::{Deserialize, Serialize};
use yolo_crypto::Sha256Digest;
use yolo_hub_terminal_protocol::PublicError;

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadFileResultFields {
    pub schema_version: u16,
    pub target: FileRootTarget,
    pub relative_path: RelativeFilePath,
    pub offset: u64,
    pub version: Sha256Digest,
    pub total_length: u64,
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
    pub eof: bool,
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ReadFileResultFields", into = "ReadFileResultFields")]
pub struct ReadFileResult(ReadFileResultFields);

impl ReadFileResult {
    pub fn new(fields: ReadFileResultFields) -> Result<Self, PublicError> {
        if fields.schema_version != FILE_PROTOCOL_SCHEMA_VERSION {
            return Err(PublicError::UnsupportedSchema);
        }
        let end = fields
            .offset
            .checked_add(fields.data.len() as u64)
            .ok_or(PublicError::InvalidPayload)?;
        if fields.data.len() > MAX_FILE_READ_BYTES as usize
            || fields.total_length > MAX_FILE_OFFSET
            || end > fields.total_length
            || fields.eof != (end == fields.total_length)
            || (fields.data.is_empty() && !fields.eof)
        {
            return Err(PublicError::InvalidPayload);
        }
        Ok(Self(fields))
    }
    pub fn fields(&self) -> &ReadFileResultFields {
        &self.0
    }
    pub fn validate_for(&self, request: &ReadFileRequest) -> Result<(), PublicError> {
        let request = request.fields();
        let result = self.fields();
        if result.target != request.target
            || result.relative_path != request.relative_path
            || result.offset != request.offset
            || request
                .expected_version
                .is_some_and(|version| version != result.version)
        {
            return Err(PublicError::TargetMismatch);
        }
        let remaining = result.total_length - result.offset;
        if result.data.len() as u64 != remaining.min(u64::from(request.length)) {
            return Err(PublicError::InvalidPayload);
        }
        Ok(())
    }
}
impl TryFrom<ReadFileResultFields> for ReadFileResult {
    type Error = PublicError;
    fn try_from(value: ReadFileResultFields) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<ReadFileResult> for ReadFileResultFields {
    fn from(value: ReadFileResult) -> Self {
        value.0
    }
}
impl std::fmt::Debug for ReadFileResult {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ReadFileResult")
            .field("target", &self.0.target)
            .field("offset", &self.0.offset)
            .field("length", &self.0.data.len())
            .field("data", &REDACTED_FILE_CONTENT)
            .finish()
    }
}
