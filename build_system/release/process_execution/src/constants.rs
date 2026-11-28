use std::time::Duration;

pub const BAZEL_REAL_ENVIRONMENT: &str = "BAZEL_REAL";
pub const HOME_ENVIRONMENT: &str = "HOME";
pub const TEMP_ENVIRONMENT: &str = "TMPDIR";
pub const PATH_ENVIRONMENT: &str = "PATH";
pub const LANG_ENVIRONMENT: &str = "LANG";
pub const LC_ALL_ENVIRONMENT: &str = "LC_ALL";
pub const ENABLED_ENVIRONMENT_VALUE: &str = "1";
pub const DISABLED_ENVIRONMENT_VALUE: &str = "0";
pub const FIXED_SYSTEM_PATH: &str = "/usr/bin:/bin:/usr/sbin:/sbin:/Library/Apple/usr/bin";
pub const DETERMINISTIC_LOCALE: &str = "C";
pub const EMPTY_HOME: &str = "/var/empty";
pub const SYSTEM_TEMPORARY_DIRECTORY: &str = "/private/tmp";
pub const NULL_DEVICE: &str = "/dev/null";
pub const DESCRIPTOR_PATH_PREFIX: &str = "/dev/fd";
pub(crate) const GIT_NO_REPLACE_OBJECTS_ENVIRONMENT: &str = "GIT_NO_REPLACE_OBJECTS";
pub(crate) const GIT_CONFIG_NOSYSTEM_ENVIRONMENT: &str = "GIT_CONFIG_NOSYSTEM";
pub(crate) const GIT_CONFIG_GLOBAL_ENVIRONMENT: &str = "GIT_CONFIG_GLOBAL";
pub(crate) const GIT_CONFIG_SYSTEM_ENVIRONMENT: &str = "GIT_CONFIG_SYSTEM";
pub(crate) const GIT_OPTIONAL_LOCKS_ENVIRONMENT: &str = "GIT_OPTIONAL_LOCKS";
pub(crate) const GIT_ATTR_NOSYSTEM_ENVIRONMENT: &str = "GIT_ATTR_NOSYSTEM";

pub(crate) const RELEASE_COMMAND_TIMEOUT: Duration = Duration::from_secs(3_600);
pub(crate) const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(10);
pub(crate) const PROCESS_TERM_GRACE: Duration = Duration::from_secs(2);
pub(crate) const PROCESS_DRAIN_GRACE: Duration = Duration::from_secs(1);
pub(crate) const PROCESS_SETTLEMENT_EPOCH_GRACE: Duration = Duration::from_secs(1);
pub(crate) const PROCESS_DROP_REAP_GRACE: Duration = PROCESS_SETTLEMENT_EPOCH_GRACE;
pub(crate) const DEFAULT_PROCESS_OUTPUT_BYTES: usize = 1024 * 1024;
pub(crate) const PROCESS_DRAIN_BUFFER_BYTES: usize = 16 * 1024;
pub(crate) const MAX_DRAIN_READS_PER_POLL: usize = 32;
pub(crate) const PROCESS_GROUP_ONLY_FILTER: u32 = 2;
pub(crate) const MAX_PROCESS_GROUP_MEMBERS: usize = 4_096;
pub(crate) const WAIT_STATUS_ERROR: libc::c_int = -1;
pub(crate) const FCNTL_ERROR: libc::c_int = -1;
pub(crate) const MAX_PROCESS_INPUT_BYTES: usize = 4 * 1024;
pub(crate) const PROCESS_DEFAULT_SIGNALS: &[libc::c_int] = &[
    libc::SIGHUP,
    libc::SIGINT,
    libc::SIGQUIT,
    libc::SIGPIPE,
    libc::SIGTERM,
];
