pub const PROGRAM: &str = yolo_launch_contract::ProcessRole::HubAttach.display_name();
pub(crate) const SOCKET_OPTION: &str = "socket";
pub(crate) const HUB_OPTION: &str = "hub";
pub(crate) const ORIGIN_OPTION: &str = "origin";
pub(crate) const TERMINAL_OPTION: &str = "terminal-id";
pub(crate) const EPOCH_OPTION: &str = "runtime-epoch";
pub(crate) const HOLDER_OPTION: &str = "holder-id";
pub(crate) const STATUS_OPTION: &str = "status-fd";
pub(crate) const COMMAND_OPTION: &str = "command-fd";
pub(crate) const COMMAND_FIELD: &str = "command_fd";
pub(crate) const LAUNCH_OPTION: &str = "launch-id";
pub(crate) const LAUNCH_FIELD: &str = "launch_id";
pub(crate) const COMMAND_FAILURE: &str = "Hub terminal command channel failed";
pub(crate) const COMMAND_DESCRIPTOR_INVALID: &str =
    "Hub terminal command descriptor or launch identity is invalid";
pub(crate) const OBSERVE_OPTION: &str = "observe";
pub(crate) const TAKEOVER_OPTION: &str = "take-control";
pub(crate) const OBSERVE_FIELD: &str = "observe";
pub(crate) const TAKEOVER_FIELD: &str = "take_control";
pub(crate) const CONTROL_ACQUIRED: &str = "control_acquired";
pub(crate) const READ_ONLY: &str = "read_only";
pub(crate) const CONTROL_LOST: &str = "control_lost";
pub(crate) const SESSION_EXITED: &str = "session_exited";
pub(crate) const NEWLINE: u8 = b'\n';
pub(crate) const INBOUND_BYTES: usize = 1;
pub(crate) const SYSCALL_FAILURE: libc::c_int = -1;
pub(crate) const IO_FAILURE: &str = "Hub terminal helper I/O failed";
pub(crate) const STATUS_FAILURE: &str =
    "Hub terminal status channel closed or received unexpected input";
pub(crate) const STATUS_DESCRIPTOR_INVALID: &str =
    "Hub terminal status descriptor is not a connected Unix stream";
