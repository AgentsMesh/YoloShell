use serde::{Deserialize, Serialize};
use yolo_hub_identity::RuntimeTarget;
use yolo_hub_terminal_protocol::{
    AttachResult, OutputChunk, TerminalHistory, TerminalSize, TerminalSnapshot,
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Attachment {
    pub target: RuntimeTarget,
    pub snapshot: Option<Snapshot>,
    pub chunks: Vec<Chunk>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history: Option<History>,
    pub next_sequence: u64,
    pub resynchronized: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Snapshot {
    pub target: RuntimeTarget,
    pub barrier_sequence: u64,
    pub size: TerminalSize,
    pub byte_length: usize,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Chunk {
    pub target: RuntimeTarget,
    pub sequence: u64,
    pub byte_length: usize,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct History {
    pub target: RuntimeTarget,
    pub barrier_sequence: u64,
    pub size: TerminalSize,
    pub byte_length: usize,
}

impl From<&OutputChunk> for Chunk {
    fn from(value: &OutputChunk) -> Self {
        let fields = value.fields();
        Self {
            target: fields.target,
            sequence: fields.sequence,
            byte_length: fields.data.len(),
        }
    }
}
impl From<&TerminalSnapshot> for Snapshot {
    fn from(value: &TerminalSnapshot) -> Self {
        let fields = value.fields();
        Self {
            target: fields.target,
            barrier_sequence: fields.barrier_sequence,
            size: fields.size,
            byte_length: fields.data.len(),
        }
    }
}
impl From<&TerminalHistory> for History {
    fn from(value: &TerminalHistory) -> Self {
        let fields = value.fields();
        Self {
            target: fields.target,
            barrier_sequence: fields.barrier_sequence,
            size: fields.size,
            byte_length: fields.data.len(),
        }
    }
}
impl From<&AttachResult> for Attachment {
    fn from(value: &AttachResult) -> Self {
        let fields = value.fields();
        Self {
            target: fields.target,
            snapshot: fields.snapshot.as_ref().map(Into::into),
            chunks: fields.chunks.iter().map(Into::into).collect(),
            history: fields.history.as_ref().map(Into::into),
            next_sequence: fields.next_sequence,
            resynchronized: fields.resynchronized,
        }
    }
}
