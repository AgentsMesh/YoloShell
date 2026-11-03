use serde::{Deserialize, Serialize};
use yolo_hub_identity::RuntimeTarget;

use crate::{
    FIRST_OUTPUT_SEQUENCE, MAX_OUTPUT_CHUNK_BYTES, MAX_SNAPSHOT_BYTES, PublicError, TerminalSize,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "TerminalSnapshotFields", into = "TerminalSnapshotFields")]
pub struct TerminalSnapshot(TerminalSnapshotFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalSnapshotFields {
    pub target: RuntimeTarget,
    pub barrier_sequence: u64,
    pub size: TerminalSize,
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

impl TerminalSnapshot {
    pub fn new(fields: TerminalSnapshotFields) -> Result<Self, PublicError> {
        fields.try_into()
    }

    pub fn fields(&self) -> &TerminalSnapshotFields {
        &self.0
    }
}

impl TryFrom<TerminalSnapshotFields> for TerminalSnapshot {
    type Error = PublicError;

    fn try_from(fields: TerminalSnapshotFields) -> Result<Self, Self::Error> {
        if fields.barrier_sequence == u64::MAX {
            return Err(PublicError::InvalidCursor);
        }
        if fields.data.len() > MAX_SNAPSHOT_BYTES {
            return Err(PublicError::InvalidPayload);
        }
        Ok(Self(fields))
    }
}

impl From<TerminalSnapshot> for TerminalSnapshotFields {
    fn from(value: TerminalSnapshot) -> Self {
        value.0
    }
}

/// One raw retained output stream from the first sequence. The client replays
/// it into a fresh emulator to recover scrollback and modes; `size` is the
/// runtime geometry at attach. Output bytes do not record resizes, so replay
/// across a resize is best effort and uses that current geometry.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "TerminalHistoryFields", into = "TerminalHistoryFields")]
pub struct TerminalHistory(TerminalHistoryFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalHistoryFields {
    pub target: RuntimeTarget,
    pub barrier_sequence: u64,
    pub size: TerminalSize,
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

impl TerminalHistory {
    pub fn new(fields: TerminalHistoryFields) -> Result<Self, PublicError> {
        fields.try_into()
    }

    pub fn fields(&self) -> &TerminalHistoryFields {
        &self.0
    }
}

impl TryFrom<TerminalHistoryFields> for TerminalHistory {
    type Error = PublicError;

    fn try_from(fields: TerminalHistoryFields) -> Result<Self, Self::Error> {
        if fields.barrier_sequence == u64::MAX {
            return Err(PublicError::InvalidCursor);
        }
        if fields.data.len() > MAX_SNAPSHOT_BYTES {
            return Err(PublicError::InvalidPayload);
        }
        Ok(Self(fields))
    }
}

impl From<TerminalHistory> for TerminalHistoryFields {
    fn from(value: TerminalHistory) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "OutputChunkFields", into = "OutputChunkFields")]
pub struct OutputChunk(OutputChunkFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputChunkFields {
    pub target: RuntimeTarget,
    pub sequence: u64,
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

impl OutputChunk {
    pub fn new(fields: OutputChunkFields) -> Result<Self, PublicError> {
        fields.try_into()
    }

    pub fn fields(&self) -> &OutputChunkFields {
        &self.0
    }
}

impl TryFrom<OutputChunkFields> for OutputChunk {
    type Error = PublicError;

    fn try_from(fields: OutputChunkFields) -> Result<Self, Self::Error> {
        if fields.sequence < FIRST_OUTPUT_SEQUENCE || fields.sequence == u64::MAX {
            return Err(PublicError::InvalidCursor);
        }
        if fields.data.is_empty() || fields.data.len() > MAX_OUTPUT_CHUNK_BYTES {
            return Err(PublicError::InvalidPayload);
        }
        Ok(Self(fields))
    }
}

impl From<OutputChunk> for OutputChunkFields {
    fn from(value: OutputChunk) -> Self {
        value.0
    }
}
