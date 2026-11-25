use std::time::Duration;

pub const COMMAND: &str = "agent";
pub const PROGRAM: &str = "yolo";
pub const SOCKET_ARGUMENT: &str = "socket";
pub const HUB_ARGUMENT: &str = "hub";

pub const NO_LOCAL_HUB: &str = "No local Hub has been started; run `yolo server start` first";
pub const SOCKET_PAIR: &str = "--socket and --hub must be supplied together";
pub const AGENT_START_UNSUPPORTED: &str = "the selected Hub cannot start this provider";
pub const INVALID_RUN_ID: &str = "the run id is not a valid non-nil UUID";
pub const CURRENT_TERMINAL_MISSING: &str =
    "YOLO_TERMINAL_ID is not set; run this command inside a Yolo terminal";
pub const CURRENT_HUB_MISMATCH: &str = "YOLO_HUB_ID does not match the selected Hub";
pub const CURRENT_WORKSPACE_INVALID: &str = "YOLO_WORKSPACE_ID is not a valid workspace id";
pub const SELECTOR_REQUIRED: &str = "a selector or --current is required";
pub const CURRENT_SELECTOR_CONFLICT: &str = "--current cannot be combined with selectors";
pub const START_CWD_UNAVAILABLE: &str = "the caller's working directory is unavailable";
pub const EXECUTION_CHANGED: &str =
    "the agent execution changed or ended; retry to resolve the current execution";
pub const TAKEN_CONTROL_HINT: &str =
    "terminal input is controlled by another client; retry with --take-control";
pub const UNEXPECTED_REPLY: &str = "the Hub returned an unexpected reply";
pub const LEASE_MISMATCH: &str = "the Hub returned a lease that does not match the request";
pub const PROMPT_INVALID: &str = "prompt is invalid";
pub const PROMPT_INPUT_LIMIT: &str = "prompt input exceeds the protocol limit";
pub const PROMPT_INPUT_TYPE: &str = "prompt input is not valid UTF-8";
pub const WAIT_TIMEOUT: &str = "timed out waiting for the selected agent state";
pub const STATE_IDLE: &str = "idle";
pub const STATE_WORKING: &str = "working";
pub const STATE_WAITING_FOR_INPUT: &str = "waiting_for_input";
pub const STATE_WAITING_FOR_APPROVAL: &str = "waiting_for_approval";
pub const STATE_EXITED: &str = "exited";
pub const STATE_UNREACHABLE: &str = "unreachable";
pub const STATE_STALE: &str = "stale";
pub const STATE_PROMPTABLE: &str = "promptable";
pub const STATE_INTERRUPTIBLE: &str = "interruptible";
pub const STATE_RESPONDABLE: &str = "respondable";
pub const INVALID_WAIT_STATE: &str = "unknown --until state";
pub const WAIT_MULTIPLE_HUBS: &str = "selectors must match agents on one Hub";

pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_REJECTED: i32 = 2;
pub const EXIT_TIMEOUT: i32 = 3;
pub const EXIT_UNKNOWN: i32 = 4;

pub const RPC_TIMEOUT: Duration = Duration::from_secs(5);
pub const CONTROL_TIMEOUT: Duration = Duration::from_secs(20);
pub const RELEASE_TIMEOUT: Duration = Duration::from_secs(2);
pub const POLL_INTERVAL: Duration = Duration::from_millis(200);
pub const REPLY_GRACE: Duration = Duration::from_secs(5);
pub const WATCH_WAIT_MS: u32 = 500;
pub const DEFAULT_TIMEOUT_SECS: u64 = 120;
pub const DEFAULT_START_TIMEOUT_SECS: u64 = 10;
pub const DEFAULT_PROMPT_WAIT_MS: u32 = 1_000;
pub const MAX_PROMPT_WAIT_MS: u32 = 1_000;
pub const MAX_TIMEOUT_SECS: u64 = 3_600;
pub const DEFAULT_PAGE_ENTRIES: u32 = 64;
pub const MAX_PAGE_ENTRIES: u32 = 256;
pub const MAX_PROMPT_STDIN_BYTES: u64 = 65_536;

pub const COMPLETED_LABEL: &str = "completed";
pub const APPROVAL_LABEL: &str = "approval";
pub const EXITED_LABEL: &str = "exited";
pub const EXECUTION_ENDED_LABEL: &str = "ended";
pub const TIMED_OUT_LABEL: &str = "timeout";
pub const INVALID_LABEL_FILTER: &str = "label filter must be key=value";
pub const DEFAULT_RUN_PAGE: u16 = 32;
pub const MAX_RUN_PAGE: u16 = 128;
pub const RUN_NOT_FOUND: &str = "agent run was not found";
pub const RUN_QUERIES_UNSUPPORTED: &str = "this Hub does not support durable agent runs";
