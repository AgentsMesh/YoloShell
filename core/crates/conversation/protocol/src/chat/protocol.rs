use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatContent<S> {
    Text {
        text: S,
    },
    ResourceLink {
        uri: S,
        name: S,
        #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
        mime_type: Option<S>,
    },
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolContent<'a> {
    Content { content: ChatContent<&'a str> },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolOutcome {
    Completed,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NoticeSeverity {
    Info,
    Warning,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(
    tag = "sessionUpdate",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ChatUpdate<'a> {
    UserMessageChunk {
        content: ChatContent<&'a str>,
    },
    AgentMessageChunk {
        content: ChatContent<&'a str>,
    },
    AgentThoughtChunk {
        content: ChatContent<&'a str>,
    },
    ToolCall {
        tool_call_id: &'a str,
        title: &'a str,
        raw_input: &'a str,
    },
    ToolCallUpdate {
        tool_call_id: &'a str,
        status: ToolOutcome,
        content: [ToolContent<'a>; 1],
    },
    CompactionUpdate {
        compaction_id: String,
        status: ToolOutcome,
        summary: [ChatContent<&'a str>; 1],
    },
    Notice {
        severity: NoticeSeverity,
        title: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<&'a str>,
    },
}
