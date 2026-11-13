use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_terminal_model::{OutputChunk, TerminalSnapshot};
use yolo_wire::ProtocolError;

pub const FORCE_SNAPSHOT_AFTER_SEQUENCE: u64 = u64::MAX;

const ATTACHMENT_SEQUENCE_INCREMENT: u64 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerAuth {
    pub token: String,
    pub controller_generation: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalRef {
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub terminal_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachRequest {
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub terminal_id: Uuid,
    #[serde(default)]
    pub after_seq: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachResult {
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub runtime_epoch: Uuid,
    pub snapshot: Option<TerminalSnapshot>,
    pub chunks: Vec<OutputChunk>,
    pub next_sequence: u64,
    pub resynchronized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputResult {
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub input_id: Uuid,
    pub duplicate: bool,
    pub delivery: String,
}

pub const fn attachment_next_sequence(sequence: u64) -> Option<u64> {
    sequence.checked_add(ATTACHMENT_SEQUENCE_INCREMENT)
}

impl AttachResult {
    pub fn validate_runtime_epoch(&self, expected: Uuid) -> Result<(), ProtocolError> {
        let snapshot_matches = self
            .snapshot
            .as_ref()
            .is_none_or(|snapshot| snapshot.runtime_epoch == expected);
        let chunks_match = self
            .chunks
            .iter()
            .all(|chunk| chunk.runtime_epoch == expected);
        if !expected.is_nil() && self.runtime_epoch == expected && snapshot_matches && chunks_match
        {
            return Ok(());
        }
        Err(ProtocolError::AttachmentMismatch)
    }

    pub fn validate_for_request(
        &self,
        expected: Uuid,
        after_sequence: u64,
    ) -> Result<u64, ProtocolError> {
        self.validate_runtime_epoch(expected)?;
        let observed_sequence = match self.snapshot.as_ref() {
            Some(snapshot)
                if self.resynchronized
                    && self.chunks.is_empty()
                    && (after_sequence == FORCE_SNAPSHOT_AFTER_SEQUENCE
                        || snapshot.barrier_sequence >= after_sequence)
                    && attachment_next_sequence(snapshot.barrier_sequence)
                        == Some(self.next_sequence) =>
            {
                snapshot.barrier_sequence
            }
            Some(_) => return Err(ProtocolError::AttachmentMismatch),
            None => self.validate_replay(after_sequence)?,
        };
        Ok(observed_sequence)
    }

    pub(crate) fn validate_replay(&self, after_sequence: u64) -> Result<u64, ProtocolError> {
        if self.resynchronized {
            return Err(ProtocolError::AttachmentMismatch);
        }
        let mut observed = after_sequence;
        for chunk in &self.chunks {
            let expected =
                attachment_next_sequence(observed).ok_or(ProtocolError::AttachmentMismatch)?;
            if chunk.sequence != expected {
                return Err(ProtocolError::AttachmentMismatch);
            }
            observed = chunk.sequence;
        }
        if attachment_next_sequence(observed) != Some(self.next_sequence) {
            return Err(ProtocolError::AttachmentMismatch);
        }
        Ok(observed)
    }
}
