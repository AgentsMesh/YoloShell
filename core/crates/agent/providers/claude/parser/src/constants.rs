pub(super) const TYPE_FIELD: &str = "type";
pub(super) const SUBTYPE_FIELD: &str = "subtype";
pub(super) const SYSTEM_RECORD: &str = "system";
pub(super) const CLEAR: &str = "clear";
pub(super) const SIDECHAIN_FIELD: &str = "isSidechain";
pub(super) const MESSAGE_FIELD: &str = "message";
pub(super) const CONTENT_FIELD: &str = "content";
pub(super) const USER_RECORD: &str = "user";
pub(super) const ASSISTANT_RECORD: &str = "assistant";
pub(super) const SUMMARY: &str = "summary";
pub(super) const FILE_HISTORY_SNAPSHOT: &str = "file-history-snapshot";
pub(super) const PROGRESS: &str = "progress";
pub(super) const QUEUE_OPERATION: &str = "queue-operation";
pub(super) const COMPACT_SUMMARY_FIELD: &str = "isCompactSummary";
pub(super) const UUID_FIELD: &str = "uuid";
pub(super) const PARENT_UUID_FIELD: &str = "parentUuid";
pub(super) const TIMESTAMP_FIELD: &str = "timestamp";
pub(super) const TEXT_FIELD: &str = "text";
pub(super) const THINKING: &str = "thinking";
pub(super) const TOOL_USE: &str = "tool_use";
pub(super) const TOOL_RESULT: &str = "tool_result";
pub(super) const TOOL_USE_ID_FIELD: &str = "tool_use_id";
pub(super) const ID_FIELD: &str = "id";
pub(super) const NAME_FIELD: &str = "name";
pub(super) const INPUT_FIELD: &str = "input";
pub(super) const IS_ERROR_FIELD: &str = "is_error";
pub(super) const IMAGE: &str = "image";
pub(super) const SOURCE_FIELD: &str = "source";
pub(super) const MEDIA_TYPE_FIELD: &str = "media_type";

/// Claude writes internal bookkeeping records alongside the conversation.
/// They are intentionally not projected and must not mark history partial.
pub(super) const INTERNAL_RECORDS: [&str; 9] = [
    "attachment",
    "last-prompt",
    "mode",
    "permission-mode",
    "atis-latch",
    "tools-used",
    "cost-state",
    "api-request-shape",
    "api-request",
];

pub(super) fn internal_record(kind: &str) -> bool {
    INTERNAL_RECORDS.contains(&kind) || kind.starts_with("api-request")
}
