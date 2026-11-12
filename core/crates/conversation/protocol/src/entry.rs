use serde::{Deserialize, Serialize};
use yolo_crypto::Sha256Digest;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationEntry {
    pub id: Sha256Digest,
    pub parent_id: Option<String>,
    pub native_id: Option<String>,
    pub timestamp: Option<String>,
    pub role: ConversationRole,
    pub parts: Vec<ConversationPart>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationRole {
    User,
    Assistant,
    Tool,
    Runtime,
}

/// Tool results remain separate records, associated by call_id even across page boundaries.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConversationPart {
    Text {
        text: String,
    },
    Thinking {
        text: String,
    },
    ToolCall {
        call_id: String,
        name: String,
        arguments: String,
    },
    ToolResult {
        call_id: String,
        output: String,
        is_error: bool,
    },
    Compaction {
        summary: String,
    },
    Notice {
        text: String,
    },
    Unsupported {
        native_kind: String,
    },
    /// Evidence exists but is not published as an asset. Never a local path or a URL.
    ImageUnavailable {
        media_type: Option<String>,
    },
}
