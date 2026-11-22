use serde::{Deserialize, Serialize};
use yolo_hub_identity::RuntimeTarget;

use crate::{
    AttachRequest, AttachSeed, FIRST_OUTPUT_SEQUENCE, MAX_ATTACH_CHUNKS, MAX_ATTACH_OUTPUT_BYTES,
    OutputChunk, OutputCursor, PublicError, TerminalHistory, TerminalSnapshot,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "AttachResultFields", into = "AttachResultFields")]
pub struct AttachResult(AttachResultFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachResultFields {
    pub target: RuntimeTarget,
    pub snapshot: Option<TerminalSnapshot>,
    pub chunks: Vec<OutputChunk>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history: Option<TerminalHistory>,
    pub next_sequence: u64,
    pub resynchronized: bool,
}

impl AttachResult {
    pub fn new(fields: AttachResultFields) -> Result<Self, PublicError> {
        fields.try_into()
    }
    pub fn fields(&self) -> &AttachResultFields {
        &self.0
    }

    pub fn validate_for_request(
        &self,
        request: AttachRequest,
    ) -> Result<OutputCursor, PublicError> {
        if request.target() != self.0.target {
            return Err(PublicError::TargetMismatch);
        }
        if self.0.history.is_some() && request.seed() != AttachSeed::History {
            return Err(PublicError::OutputGap);
        }
        let observed = self.0.next_sequence - FIRST_OUTPUT_SEQUENCE;
        if let Some(history) = self.0.history.as_ref() {
            let history = history.fields();
            if history.target != self.0.target {
                return Err(PublicError::TargetMismatch);
            }
            if request
                .cursor()
                .is_some_and(|cursor| history.barrier_sequence < cursor.after_sequence)
            {
                return Err(PublicError::OutputGap);
            }
            return Ok(OutputCursor {
                target: self.0.target,
                after_sequence: history.barrier_sequence,
            });
        }
        match (self.0.snapshot.as_ref(), request.cursor()) {
            (Some(snapshot), Some(cursor))
                if snapshot.fields().barrier_sequence < cursor.after_sequence =>
            {
                return Err(PublicError::OutputGap);
            }
            (None, None) => return Err(PublicError::OutputGap),
            (None, Some(cursor)) => {
                let expected_first = cursor.after_sequence + FIRST_OUTPUT_SEQUENCE;
                let actual_first = self
                    .0
                    .chunks
                    .first()
                    .map_or(self.0.next_sequence, |chunk| chunk.fields().sequence);
                if expected_first != actual_first {
                    return Err(PublicError::OutputGap);
                }
            }
            _ => {}
        }
        Ok(OutputCursor {
            target: self.0.target,
            after_sequence: observed,
        })
    }
}

impl TryFrom<AttachResultFields> for AttachResult {
    type Error = PublicError;

    fn try_from(fields: AttachResultFields) -> Result<Self, Self::Error> {
        if fields.next_sequence < FIRST_OUTPUT_SEQUENCE {
            return Err(PublicError::InvalidCursor);
        }
        if fields.chunks.len() > MAX_ATTACH_CHUNKS {
            return Err(PublicError::InvalidPayload);
        }
        if let Some(history) = fields.history.as_ref() {
            let history = history.fields();
            if fields.snapshot.is_some()
                || !fields.chunks.is_empty()
                || fields.resynchronized
                || history.target != fields.target
                || history.barrier_sequence + FIRST_OUTPUT_SEQUENCE != fields.next_sequence
            {
                return Err(PublicError::OutputGap);
            }
        } else if let Some(snapshot) = fields.snapshot.as_ref() {
            let snapshot = snapshot.fields();
            if snapshot.target != fields.target {
                return Err(PublicError::TargetMismatch);
            }
            if !fields.resynchronized
                || !fields.chunks.is_empty()
                || snapshot.barrier_sequence + FIRST_OUTPUT_SEQUENCE != fields.next_sequence
            {
                return Err(PublicError::OutputGap);
            }
        } else {
            if fields.resynchronized {
                return Err(PublicError::OutputGap);
            }
            let mut bytes = 0;
            let mut previous = None;
            for chunk in &fields.chunks {
                let chunk = chunk.fields();
                if chunk.target != fields.target {
                    return Err(PublicError::TargetMismatch);
                }
                if previous
                    .is_some_and(|sequence| sequence + FIRST_OUTPUT_SEQUENCE != chunk.sequence)
                {
                    return Err(PublicError::OutputGap);
                }
                bytes += chunk.data.len();
                previous = Some(chunk.sequence);
            }
            if bytes > MAX_ATTACH_OUTPUT_BYTES {
                return Err(PublicError::InvalidPayload);
            }
            if previous
                .is_some_and(|sequence| sequence + FIRST_OUTPUT_SEQUENCE != fields.next_sequence)
            {
                return Err(PublicError::OutputGap);
            }
        }
        Ok(Self(fields))
    }
}

impl From<AttachResult> for AttachResultFields {
    fn from(value: AttachResult) -> Self {
        value.0
    }
}
