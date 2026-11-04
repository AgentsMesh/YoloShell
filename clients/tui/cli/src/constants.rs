pub use yolo_launch_contract::EXECUTABLE_NAME as PROGRAM_NAME;
pub use yolo_tui_audit::DEFAULT_AUDIT_LIMIT;
pub use yolo_tui_audit::NO_LOCAL_HUB;
pub const ABOUT: &str =
    "Persistent terminal tabs. Run yolo to enter a shell; start any agent there.";
pub const CONTROLS: &str = concat!(
    "Ctrl-B c opens a tab with a shell; v or % splits right; - or \" splits down.\n",
    "Ctrl-B n/p switches tabs; o or arrows focuses a pane; w opens the tab manager.\n",
    "Ctrl-B s focuses the Agent runs/Hubs sidebar; b toggles it; g opens the existing-terminal picker.\n",
    "Ctrl-B x closes a pane view; X closes a tab view. Shells keep running; g reopens them.\n",
    "Ctrl-B R opens focused-terminal recovery; Ctrl-B Ctrl-B sends a literal Ctrl-B.\n",
    "Ctrl-B m switches an agent pane between Terminal and Chat; Escape returns to Terminal.\n",
    "Ctrl-B d detaches. Ordinary keys (including q) go to the shell.\n",
    "Detaching leaves shells and agents running. An agent can finish while its shell remains usable.\n",
    "Also: yolo agent --help · yolo protocol describe · yolo conversation --help."
);
pub const TERMINALS_COMMAND: &str = "terminals";
pub const CATALOG_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
pub const INVALID_CWD: &str =
    "Shell directory must be an absolute path or ~, without control characters";
pub const REMOTE_HOME: &str = "~";
pub const TERMINAL_HEADER: &str = "TERMINAL\tSTATE\tNAME";
/// Terminate replies can precede the catalogue's settled exit row.
pub const LIFECYCLE_SETTLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);
pub const LIFECYCLE_SETTLE_POLL: std::time::Duration = std::time::Duration::from_millis(100);
pub const NO_MATCHING_TERMINAL: &str = "No terminal matches the selector";
pub const AMBIGUOUS_TERMINAL: &str =
    "Several terminals match; use a terminal UUID or HUB/TERMINAL address";
pub const UNEXPECTED_LIFECYCLE_REPLY: &str = "Hub returned an unexpected lifecycle reply";
pub const ARCHIVED_STATE: &str = "Archived";
pub const CLIENT_HEADER: &str = "CLIENT_ID\tKIND";
pub const CLIENT_KIND_PRIMARY: &str = "primary";
pub const CLIENT_KIND_CLIENT: &str = "client";
pub const SOCKET_ARGUMENT: &str = "socket";
pub const ENDPOINT_ARGUMENT: &str = "endpoint";
pub const HUB_ARGUMENT: &str = "hub";
pub const LOCAL_PROFILE_ARGUMENT: &str = "local_profile";
pub const REMOTE_PROFILE_ARGUMENT: &str = "remote_profile";
pub const ACCOUNT_KEY_ARGUMENT: &str = "account_key";
pub const CLIENT_KEY_ARGUMENT: &str = "client_key";
pub const MAX_PRIVATE_KEY_BYTES: u64 = 64 * 1024;
pub const READ_LIMIT_SENTINEL: u64 = 1;
pub const INVALID_REMOTE_PROFILE: &str = "Remote Hub profile is invalid";
pub const INVALID_REMOTE_CREDENTIAL: &str =
    "Remote Hub requires valid unencrypted SSH account and Ed25519 client keys";
pub const PRIVATE_REMOTE_FILE_REQUIRED: &str =
    "Remote profile and keys must be bounded private files owned by the current user";
pub const CONNECTION_GROUP: &str = "connection";
pub const INVALID_PATH: &str = "Local profile paths must contain valid non-control text";
pub(crate) const INVALID_LAYOUT: &str = "Layout identity must be a non-nil UUID";
pub const PLATFORM_SNAPSHOT_UNSUPPORTED: &str = "this Hub does not advertise platform snapshots";
pub const PLATFORM_EVENTS_UNSUPPORTED: &str = "this Hub does not advertise platform events";
pub const PLATFORM_EVENTS_SUBSCRIBE_UNSUPPORTED: &str =
    "this Hub does not advertise event subscriptions";
pub const UNEXPECTED_EVENT_SUBSCRIPTION_REPLY: &str = "Hub returned a non-subscription reply";
pub const UNEXPECTED_EVENTS_REPLY: &str = "Hub returned an unexpected platform event reply";
pub const TERMINAL_TEXT_UNSUPPORTED: &str = "this Hub does not advertise terminal text reads";
pub const TERMINAL_TEXT_NO_RUNTIME: &str = "the selected terminal has no live runtime";
pub const UNEXPECTED_TEXT_REPLY: &str = "Hub returned an unexpected terminal text reply";
pub const UNEXPECTED_SNAPSHOT_REPLY: &str = "Hub returned an unexpected snapshot reply";
pub const DEFAULT_SNAPSHOT_LIMIT: u16 = 128;
pub const MAX_SNAPSHOT_LIMIT: u16 = 128;
pub const DEFAULT_EVENTS_LIMIT: u16 = 64;
pub const MAX_EVENTS_LIMIT: u16 = 256;
pub const AUDIT_LOCAL_ONLY: &str = "audit reads require the local profile";
pub const MANAGEMENT_LOCAL_ONLY: &str =
    "Hub management requires the local entry Hub (use --local-profile or --socket/--hub)";
pub const MANAGEMENT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
/// Pairing spans an SSH handshake and a remote configuration activation.
pub const ENROLLMENT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

pub const RECIPE_SCHEMA_VERSION: u16 = 1;
pub const SCHEMA_VERSION_FIELD: &str = "schema_version";
pub const MAX_RECIPE_AGENTS: usize = 8;
pub const MAX_RECIPE_BYTES: u64 = 64 * 1024;
pub const MAX_RECIPE_PROVIDER_BYTES: usize = 96;
pub const MAX_RECIPE_NAME_BYTES: usize = 128;
pub const MAX_RECIPE_PATH_BYTES: usize = 4096;
pub const DEFAULT_RECIPE_WAIT_SECONDS: u32 = 60;
pub const MAX_RECIPE_WAIT_SECONDS: u32 = 600;
pub const DEFAULT_RECIPE_PROMPT_SECONDS: u32 = 60;
pub const MAX_RECIPE_PROMPT_SECONDS: u32 = 600;
pub const RECIPE_WAIT_POLL: std::time::Duration = std::time::Duration::from_millis(100);
pub const RECIPE_WAIT_PROMPTABLE: &str = "promptable";
pub const RECIPE_WAIT_IDLE: &str = "idle";
pub const RECIPE_WAIT_EXITED: &str = "exited";
pub const INVALID_RECIPE: &str = "recipe file is not valid recipe JSON or exceeds 64 KiB";
pub const INVALID_RECIPE_SCHEMA: &str = "recipe schema_version must be 1";
pub const INVALID_RECIPE_AGENTS: &str = "recipe must list between 1 and 8 agents";
pub const INVALID_RECIPE_CWD: &str = "recipe requires an absolute cwd when no workspace is set";
pub const INVALID_RECIPE_PROVIDER: &str =
    "recipe agent provider is empty, too long or contains control characters";
pub const INVALID_RECIPE_NAME: &str = "recipe agent names must be unique and non-empty";
pub const INVALID_RECIPE_TOKEN: &str = "recipe start_token must be a non-nil UUID";
pub const INVALID_RECIPE_PARENT: &str = "recipe parent must be a non-empty run name or UUID";
pub const INVALID_RECIPE_LABELS: &str = "recipe labels are invalid or exceed the run label bounds";
pub const INVALID_RECIPE_DAG: &str =
    "recipe agent dependencies must reference unique named agents without cycles";
pub const INVALID_RECIPE_WAIT: &str =
    "recipe wait requires after dependencies and a timeout between 1 and 600 seconds";
pub const INVALID_RECIPE_PROMPT: &str = "recipe prompt must be bounded text without terminal controls and a timeout between 1 and 600 seconds";
pub const RECIPE_PROMPT_REQUIRES_RUN: &str = "recipe prompt requires a durable run";
pub const RECIPE_PROMPT_REJECTED: &str = "recipe prompt was not delivered";
pub const RECIPE_PROMPT_TURN_TIMEOUT: &str = "recipe prompt turn did not end before the timeout";
pub const RECIPE_PROMPT_TARGET_MISSING: &str = "recipe prompt run has no observed execution";
pub const RECIPE_PROMPT_DELIVERED: &str = "delivered";
pub const RECIPE_PROMPT_TURN_END: &str = "turn_end";
pub const RECIPE_DEPENDENCY_RUN_MISSING: &str = "recipe wait dependency has no durable run";
pub const RECIPE_WAIT_TIMEOUT: &str = "recipe wait timed out";
pub const RECIPE_WAIT_RUN_STOPPED: &str = "recipe wait run was stopped before reaching the state";
pub const RECIPE_WAIT_EXECUTION_ENDED: &str =
    "recipe wait execution ended before reaching the state";
pub const UNEXPECTED_RECIPE_WAIT_REPLY: &str = "Hub returned an unexpected recipe wait reply";
pub const RECIPE_LABELS_REQUIRE_RUN: &str = "recipe labels require a durable run";
pub const UNEXPECTED_RECIPE_REPLY: &str = "Hub returned an unexpected recipe reply";
