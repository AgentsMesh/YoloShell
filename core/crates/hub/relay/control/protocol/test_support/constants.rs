pub const LAUNCH_ID: &str = "00000000-0000-4000-8000-000000000001";
pub const HUB_ID: &str = "00000000-0000-4000-8000-000000000002";
pub const SESSION_ID: &str = "00000000-0000-4000-8000-000000000003";
pub const RUNTIME_EPOCH: &str = "00000000-0000-4000-8000-000000000004";
pub const AGENT_ID: &str = "00000000-0000-4000-8000-000000000005";
pub const EXECUTION_ID: &str = "00000000-0000-4000-8000-000000000006";
pub const STREAM_ID: &str = "00000000-0000-4000-8000-000000000007";
pub const REQUEST_ID: &str = "00000000-0000-4000-8000-000000000008";
pub const ALTERNATE_ID: &str = "abcdefab-cdef-4abc-8def-abcdefabcdef";
pub const NIL_ID: &str = "00000000-0000-0000-0000-000000000000";
pub const PRIVATE_PAYLOAD: &str = "private-provider-content";
pub const EXTRA_FIELD: &str = "unsupported_field";
pub const EXTRA_VALUE: &str = "unexpected";
pub const UNSUPPORTED_SCHEMA: u16 = u16::MAX;
pub const INVALID_SEQUENCE: u64 = 0;
pub const RECEIPT_OUTCOME: &str = "receipt";
pub const CONTROL_ACTION_FIELD: &str = "action";
pub const CONTROL_CURSOR_FIELD: &str = "expected_cursor";
pub const CONTROL_TARGET_FIELD: &str = "target";

pub mod path {
    pub const LAUNCH: &str = "/launch_id";
    pub const REQUEST_ID: &str = "/request/request_id";
    pub const CURSOR_STREAM: &str = "/request/expected_cursor/stream";
    pub const CURSOR_SEQUENCE: &str = "/request/expected_cursor/sequence";
    pub const CURSOR_EXECUTION: &str = "/request/expected_cursor/target/execution_id";
    pub const RECEIPT_ID: &str = "/outcome/receipt/request_id";
    pub const RECEIPT_EXECUTION: &str = "/outcome/receipt/target/execution_id";
    pub const TERMINAL_HUB: &str = "/terminal/address/hub_id";
    pub const SESSION: &str = "/terminal/address/terminal_id";
    pub const EPOCH: &str = "/terminal/runtime_epoch";
    pub const AGENT: &str = "/request/target/address/agent_id";
    pub const EXECUTION: &str = "/request/target/execution_id";
    pub const SCHEMA: &str = "/schema_version";
    pub const REQUEST_SCHEMA: &str = "/request/schema_version";
}
