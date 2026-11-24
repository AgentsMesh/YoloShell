pub const SCHEMA_VERSION: u16 = 2;
const ENVELOPE_BUDGET_BYTES: usize = 4_096;
/// Maximum Cap'n Proto payload bytes, excluding the network-order u32 length.
pub const MAX_FRAME_BYTES: usize =
    ENVELOPE_BUDGET_BYTES + yolo_hub_agent_protocol::MAX_PROMPT_BYTES;
pub const FRAME_PREFIX_BYTES: usize = std::mem::size_of::<u32>();
pub const MAX_HOLDER_BYTES: usize = 256;
pub const MAX_STATUS_BYTES: usize = 512;
pub(crate) const MAX_TRAVERSAL_WORDS: usize = MAX_FRAME_BYTES;
pub(crate) const MAX_NESTING: i32 = 32;
pub(crate) const SCHEMA_ERROR: &str = "unsupported relay control schema";
pub(crate) const IDENTITY_ERROR: &str = "invalid relay launch identity";
pub(crate) const REQUEST_ERROR: &str = "invalid relay control request";
pub(crate) const ACTION_ERROR: &str = "relay control supports only Interrupt and SubmitPrompt";
pub(crate) const ORIGIN_ERROR: &str = "relay control target belongs to another origin";
pub(crate) const RECEIPT_ERROR: &str = "relay control receipt does not match its request";
pub(crate) const RESPONSE_ERROR: &str = "relay control reply does not echo the exact request";
pub(crate) const EMPTY_ERROR: &str = "relay control payload is empty";
pub(crate) const SIZE_ERROR: &str = "relay control payload exceeds the byte limit";
pub(crate) const FRAME_ERROR: &str = "relay payload is not a valid single Cap'n Proto message";

pub mod field {
    pub const SCHEMA_VERSION: &str = "schema_version";
    pub const LAUNCH_ID: &str = "launch_id";
    pub const TERMINAL: &str = "terminal";
    pub const REQUEST: &str = "request";
    pub const OUTCOME: &str = "outcome";
    pub const RECEIPT: &str = "receipt";
    pub const REASON: &str = "reason";
}
