use std::time::Duration;

pub(crate) use crate::constants::{
    MAX_RUNTIME_BINARY_BYTES, MAX_RUNTIME_PATH_BYTES as MAX_REMOTE_PATH_BYTES,
    RUNTIME_EXECUTABLE_FILE_NAME, RUNTIME_EXECUTABLE_MODE, RUNTIME_RELEASE_PREFIX,
};

pub const DEFAULT_SSH_PORT: u16 = 22;
pub const MAX_COMMAND_ARGUMENTS: usize = 128;
pub const MAX_COMMAND_OUTPUT_BYTES: usize = 1_048_576;
pub const MAX_COMMAND_TIMEOUT: Duration = Duration::from_secs(30);
pub const MAX_FILE_READ_BYTES: u64 = 8_388_608;
pub const MAX_CHANNEL_APPROVAL_BYTES: usize = 1_048_576;
pub const MAX_CHANNEL_APPROVAL_MESSAGES: usize = 256;
pub const SFTP_CHANNEL_BRIDGE_BYTES: usize = 65_536;
pub const RUNTIME_UPLOAD_CHUNK_BYTES: usize = 65_536;
pub const REMOTE_FILESYSTEM_DETAIL_MAX_CHARACTERS: usize = 512;
pub const SSH_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
pub const SSH_INACTIVITY_TIMEOUT: Duration = Duration::from_secs(60);
pub const SSH_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(15);
pub const SSH_KEEPALIVE_MAX_FAILURES: usize = 3;
pub const SSH_OPERATION_TIMEOUT: Duration = crate::session_timing::REMOTE_SSH_OPERATION_TIMEOUT;
pub const SSH_CLEANUP_PHASE_COUNT: u32 = 3;
pub const SSH_CLEANUP_SETTLEMENT_MARGIN: Duration = Duration::from_secs(1);
pub const SSH_CLEANUP_ABORT_SETTLEMENT_TIMEOUT: Duration = Duration::from_secs(1);
pub const SSH_CLEANUP_RESERVATION_INCREMENT: usize = 1;
pub const FIRST_CHANNEL_ID: u64 = 1;
pub const MAX_KNOWN_HOSTS_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_KNOWN_HOSTS_LINE_BYTES: usize = 16 * 1024;
pub const MAX_KNOWN_HOSTS_ENTRIES: usize = 65_536;
pub const MAX_KNOWN_HOSTS_LINES: usize = 131_072;
pub const MAX_HOST_PATTERNS_PER_ENTRY: usize = 256;
pub const MAX_HOST_PATTERN_BYTES: usize = 4_096;
pub const MAX_REMOTE_USER_BYTES: usize = 256;
pub const MAX_REMOTE_HOST_BYTES: usize = 253;
pub const MAX_IDENTITY_FILE_BYTES: usize = 1024 * 1024;
pub const MAX_SSH_IDENTITY_FILES: usize = 32;
pub const MAX_SSH_AGENT_IDENTITIES: usize = 256;
pub const MAX_SSH_AGENT_FRAME_BYTES: usize = 256 * 1024;
pub const SSH_AGENT_LENGTH_PREFIX_BYTES: usize = std::mem::size_of::<u32>();
pub const SSH_AGENT_SINGLE_MESSAGE_BYTES: u32 = 1;
pub const SSH_AGENT_REQUEST_IDENTITIES_MESSAGE: u8 = 11;
pub const SSH_AGENT_IDENTITIES_ANSWER_MESSAGE: u8 = 12;
pub const SSH_AUTH_SOCK_ENVIRONMENT: &str = "SSH_AUTH_SOCK";
#[cfg(any(test, feature = "broker-test-support"))]
pub const SSH_EXTENDED_DATA_STDERR: u32 = 1;
pub const KNOWN_HOSTS_DENIED_MODE: u32 = 0o022;
pub const IDENTITY_DENIED_MODE: u32 = 0o077;
pub const LOCAL_FILE_PERMISSION_MASK: u32 = 0o777;

pub const RUNTIME_CLIENT_NAME: &str = "yolo-remote";
pub const RUNTIME_SERVER_NAME: &str = "yolo-host";
pub const BRIDGE_COMMAND: &str = "bridge";
pub const STDIO_ARGUMENT: &str = "--stdio";
pub const HOME_ARGUMENT: &str = "--home";
pub const SFTP_SUBSYSTEM: &str = "sftp";
pub const FSYNC_EXTENSION: &str = "fsync@openssh.com";
pub const FSYNC_EXTENSION_VERSION: &str = "1";
pub const POSIX_RENAME_EXTENSION: &str = "posix-rename@openssh.com";
pub const POSIX_RENAME_VERSION: &str = "1";
pub const SHELL_SEPARATOR: &str = " ";
pub const SHELL_PROGRAM_SLOT_COUNT: usize = 1;
pub const SHELL_SINGLE_QUOTE_ESCAPE: &str = "'\"'\"'";
pub const SHELL_CHANGE_DIRECTORY: &str = "cd --";
pub const SHELL_AND_EXEC: &str = "&& exec";
pub const ROOT_PATH: &str = "/";
pub const PATH_SEPARATOR: char = '/';
pub const NUL_CHARACTER: char = '\0';
pub const CURRENT_PATH_COMPONENT: &str = ".";
pub const PARENT_PATH_COMPONENT: &str = "..";
pub const KNOWN_HOSTS_COMMENT_PREFIX: char = '#';
pub const KNOWN_HOSTS_MARKER_PREFIX: char = '@';
pub const KNOWN_HOSTS_REVOKED_MARKER: &str = "@revoked";
pub const KNOWN_HOSTS_CA_MARKER: &str = "@cert-authority";
pub const KNOWN_HOST_PORT_PREFIX: char = '[';
pub const KNOWN_HOST_PORT_SEPARATOR: &str = "]:";
pub const HASHED_HOST_PREFIX: &str = "|1|";
pub const HASHED_HOST_SEPARATOR: char = '|';
pub const NEGATED_HOST_PREFIX: char = '!';
pub const OPERATION_TIMEOUT_MESSAGE: &str = "SSH channel operation timed out";
pub const CHANNEL_OPEN_TASK_FAILURE_CONTEXT: &str = "SSH channel-open task failed";
pub const CHANNEL_OPEN_ABORT_SETTLEMENT_MESSAGE: &str =
    "aborted SSH channel-open task did not settle";
pub const SFTP_CHANNEL_ABORT_SETTLEMENT_MESSAGE: &str =
    "aborted SSH SFTP channel task did not settle";
pub const CHANNEL_CLOSE_CLEANUP_TIMEOUT_MESSAGE: &str = "SSH channel close cleanup timed out";
pub const SFTP_CHANNEL_DRIVER_TASK_FAILURE_CONTEXT: &str = "SSH SFTP channel driver task failed";
pub const SFTP_CHANNEL_OPEN_FAILURE_MESSAGE: &str =
    "SSH SFTP channel received open failure after confirmation";
pub const SFTP_CHANNEL_RECEIVER_EOF_MESSAGE: &str =
    "SSH SFTP channel receiver ended without channel close";
pub const SSH_DISCONNECT_CLEANUP_TIMEOUT_MESSAGE: &str = "SSH disconnect cleanup timed out";
pub const SSH_DISCONNECT_DESCRIPTION: &str = "";
pub const SSH_DISCONNECT_LANGUAGE_TAG: &str = "";
pub const CLEANUP_REGISTRY_CLOSED_MESSAGE: &str = "SSH cleanup registry is settling";
pub const CLEANUP_REGISTRY_EXHAUSTED_MESSAGE: &str = "SSH cleanup registry is exhausted";
pub const CLEANUP_REGISTRY_FAILED_MESSAGE: &str = "SSH cleanup did not settle successfully";
pub const CLEANUP_REGISTRY_TIMEOUT_MESSAGE: &str = "SSH cleanup settlement timed out";
pub const COMMAND_KILL_CLEANUP_TIMEOUT_MESSAGE: &str = "SSH command kill cleanup timed out";
pub const TEMPORARY_PATH_CLEANUP_TIMEOUT_MESSAGE: &str = "SSH temporary path cleanup timed out";
pub const CHANNEL_CLOSED_MESSAGE: &str = "SSH channel closed before replying to its request";
pub const UNEXPECTED_CHANNEL_REPLY_MESSAGE: &str =
    "unexpected SSH channel message before request approval";
pub const COMMAND_DATA_AFTER_EOF_MESSAGE: &str =
    "SSH command channel sent output after end-of-stream";
pub const SSH_AGENT_FAILURE_CONTEXT: &str = "SSH agent failed";
pub const SSH_IDENTITY_FAILURE_CONTEXT: &str = "SSH identity failed";
pub const SSH_IDENTITY_UTF8_FAILURE_CONTEXT: &str = "SSH identity is not UTF-8";
pub const SSH_IDENTITY_DECODE_FAILURE_CONTEXT: &str = "SSH identity decoding failed";
pub const SSH_AGENT_IDENTITY_LIMIT_CONTEXT: &str = "SSH agent identity limit reached";
pub const SSH_AGENT_PROTOCOL_MESSAGE: &str = "SSH agent returned an invalid identities response";
pub const SSH_CERTIFICATE_ALGORITHM_SUFFIX: &str = "-cert-v01@openssh.com";
pub const REMOTE_REQUEST_FAILED_MESSAGE: &str = "request failed";
pub const FSYNC_REQUIRED_MESSAGE: &str = "server lacks durable SFTP file sync support";
pub const POSIX_RENAME_UNEXPECTED_PACKET_MESSAGE: &str =
    "POSIX rename returned an unexpected packet";
pub const DIRECTORY_DURABILITY_WARNING: &str =
    "SFTP cannot fsync the deployment directory; directory-entry durability depends on the server";
pub const RUNTIME_UPLOAD_PREFIX: &str = ".yolo-host-upload-";
pub const RUNTIME_LINK_PREFIX: &str = ".yolo-host-link-";
pub const POSIX_RENAME_SESSION_SCOPE: &str = "posix-rename-session";
pub const SFTP_CAPABILITY_SESSION_SCOPE: &str = "sftp-capability-session";
pub const RUNTIME_FILE_SEPARATOR: &str = "-";
pub const RUNTIME_VERSION_ARGUMENT: &str = "--version";
pub const ARCHITECTURE_PROGRAM: &str = "/usr/bin/uname";
pub const ARCHITECTURE_ARGUMENT: &str = "-m";
pub const PLATFORM_ARGUMENT: &str = "-s";
pub const ARCHITECTURE_AARCH64: &str = "aarch64";
pub const ARCHITECTURE_ARM64: &str = "arm64";
pub const ARCHITECTURE_X86_64: &str = "x86_64";
pub const ARCHITECTURE_AMD64: &str = "amd64";
pub const PLATFORM_DARWIN: &str = "Darwin";
pub const PLATFORM_LINUX: &str = "Linux";
pub const PLATFORM_MACOS_PATH: &str = "macos";
pub const PLATFORM_LINUX_PATH: &str = "linux";
pub const SUCCESS_EXIT_STATUS: u32 = 0;
pub const MACHO_CPU_OFFSET: usize = 4;
pub const SHA256_HEX_LENGTH: usize = 64;
pub const VERSION_MAX_BYTES: usize = 64;
pub const RUNTIME_UPLOAD_MODE: u32 = 0o600;
pub const RUNTIME_DIRECTORY_MODE: u32 = 0o700;
pub const FILE_PERMISSION_MASK: u32 = 0o777;
pub const SFTP_PATH_LENGTH_BYTES: usize = 4;
pub const ELF_HEADER_BYTES: usize = 20;
pub const ELF_MAGIC: &[u8] = b"\x7fELF";
pub const ELF_ENDIAN_OFFSET: usize = 5;
pub const ELF_CLASS_OFFSET: usize = 4;
pub const ELF_VERSION_OFFSET: usize = 6;
pub const ELF_MACHINE_OFFSET: usize = 18;
pub const ELF_CLASS_64: u8 = 2;
pub const ELF_VERSION_CURRENT: u8 = 1;
pub const ELF_LITTLE_ENDIAN: u8 = 1;
pub const ELF_BIG_ENDIAN: u8 = 2;
pub const ELF_MACHINE_X86_64: u16 = 62;
pub const ELF_MACHINE_AARCH64: u16 = 183;
pub const MACHO_HEADER_BYTES: usize = 8;
pub const MACHO_64_LITTLE_MAGIC: [u8; 4] = [0xcf, 0xfa, 0xed, 0xfe];
pub const MACHO_64_BIG_MAGIC: [u8; 4] = [0xfe, 0xed, 0xfa, 0xcf];
pub const MACHO_CPU_X86_64: u32 = 0x0100_0007;
pub const MACHO_CPU_ARM64: u32 = 0x0100_000c;

pub const REQUIRED_RUNTIME_CAPABILITIES: &[&str] = &[
    crate::protocol::capability::DURABLE_PTY,
    crate::protocol::capability::HOST_ADOPTION,
    crate::protocol::capability::CAPNP_BINARY,
    crate::protocol::capability::OUTPUT_JOURNAL,
    crate::protocol::capability::SSH_STDIO_BRIDGE,
    crate::protocol::capability::WRITER_LEASE_V2,
];

#[cfg(test)]
pub const TEST_STATUS_ARGUMENT: &str = "status";
#[cfg(test)]
pub const TEST_PATH_FILL: &str = "x";
#[cfg(test)]
pub const TEST_KNOWN_HOSTS_FILE_NAME: &str = "known_hosts";
