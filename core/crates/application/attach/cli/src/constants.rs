use std::time::Duration;

pub const CONTROL_STATUS: &str = "yolo internal attach: control acquired";
pub const READ_ONLY_STATUS: &str = "yolo internal attach: read only";
pub const CONTROL_LOST_STATUS: &str = "yolo internal attach: control lost; continuing read only";
pub const HOST_STARTED_STATUS: &str = "yolo internal attach: started host";
pub const SESSION_RESUMED_STATUS: &str = "yolo internal attach: session resumed";
pub const SESSION_ENDING_STATUS: &str = "yolo internal attach: session ending";
pub const SESSION_EXITED_STATUS: &str = "yolo internal attach: session exited";
pub const UNKNOWN_HOLDER_STATUS: &str = "unknown";
pub const STATUS_FD_RESERVED: &str = "yolo internal attach status descriptor must not be negative or replace standard input/output/error";
pub const STATUS_FD_OPEN_CONTEXT: &str =
    "validate inherited yolo internal attach status descriptor";
pub const STATUS_FD_SOCKET_TYPE_CONTEXT: &str =
    "read inherited yolo internal attach status descriptor socket type";
pub const STATUS_FD_SOCKET_TYPE_LENGTH: &str =
    "inherited yolo internal attach status descriptor returned an invalid socket type length";
pub const STATUS_FD_NOT_STREAM: &str =
    "inherited yolo internal attach status descriptor is not a stream socket";
pub const STATUS_FD_PEER_CONTEXT: &str =
    "validate inherited yolo internal attach status descriptor connection";
pub const STATUS_FD_PEER_ADDRESS_LENGTH: &str =
    "inherited yolo internal attach status descriptor returned an invalid peer address";
pub const STATUS_FD_NOT_UNIX: &str =
    "inherited yolo internal attach status descriptor is not a Unix-domain socket";
pub const STATUS_FD_FLAGS_CONTEXT: &str =
    "secure inherited yolo internal attach status descriptor flags";
pub const STATUS_FD_STREAM_CONTEXT: &str = "adopt inherited yolo internal attach status descriptor";
pub const STATUS_PEER_CLOSED: &str = "yolo internal attach status peer closed";
pub const STATUS_PEER_INPUT: &str = "yolo internal attach status peer sent unexpected data";
pub const STATUS_WRITE_TIMEOUT: Duration = Duration::from_secs(1);
pub const STATUS_WRITE_TIMEOUT_MESSAGE: &str = "yolo internal attach status write timed out";
pub const HUMAN_STATUS_WRITE_CONTEXT: &str = "write yolo internal attach terminal status";
pub const NDJSON_DELIMITER: u8 = b'\n';
#[cfg(test)]
pub const REMOVED_STATUS_SOCKET_ARGUMENT: &str = "--status-socket";
pub(super) const HOME_FIELD: &str = "home";
pub(super) const SOCKET_FIELD: &str = "socket";
pub(super) const BROKER_SOCKET_FIELD: &str = "broker_socket";
pub(super) const HOST_EXECUTABLE_FIELD: &str = "host_executable";
