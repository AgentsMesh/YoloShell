pub const COMMANDS: &[&str] = &["workspace", "worktree"];
pub const PROGRAM: &str = "yolo";
pub const SOCKET_PAIR: &str = "--socket and --hub must be supplied together";
pub const NO_LOCAL_HUB: &str = "No local Hub has been started; run `yolo start` first";
pub const INVALID_SELECTOR: &str = "workspace selector must be a workspace id or name";
pub const INVALID_LABEL: &str = "workspace labels must be key=value";
pub const MIXED_MEMBERSHIP: &str =
    "one update cannot both bind and unbind members; send separate updates";
pub const INVALID_WORKTREE: &str = "worktree selector must name a worktree workspace";
pub const TRUST_REQUIRED: &str =
    "creating a worktree requires --trust-repository (git hooks execute code)";
pub const REMOVED: &str = "removed worktree workspace";
pub const DEFAULT_LIMIT: u16 = 64;
pub const MAX_LIMIT: u16 = 128;
pub const RPC_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
