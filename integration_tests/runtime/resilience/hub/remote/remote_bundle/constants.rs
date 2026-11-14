use std::time::Duration;

pub use yolo_distribution_publication_protocol::{
    DIRECTORY_MODE, EXECUTABLE_MODE, INSTALL_BUNDLE_ARGUMENT, INSTALL_ROOT_ARGUMENT, MANIFEST_MODE,
    REQUEST_CAPNP_ARGUMENT,
};
pub use yolo_remote_provisioning_protocol::REMOTE_BOOTSTRAP_ARGUMENT;
pub const ROOT_NAME: &str = "runtime-install";
pub const STATE_NAME: &str = "remote-hub";
pub const HOME_NAME: &str = "remote-home";
pub const CONSUMER_STATE: &str = "consumer-hub";
pub const CONSUMER_HOME: &str = "consumer-home";
pub const SOURCE_REVISION: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub const VERSION: &str = "0.0.0-acceptance";
pub const RUNTIME_VERSION_ARGUMENT: &str = "--version";
pub const BUNDLE_HOST_BINARY_ENV: &str = "YOLO_TEST_BUNDLE_HOST_BINARY";
pub const MAX_CAPTURE_BYTES: u64 = 64 * 1024;
pub const CAPTURE_SENTINEL: u64 = 1;
pub const TEST_TIMEOUT: Duration = Duration::from_secs(60);
pub const PROBE_TIMEOUT: Duration = Duration::from_millis(250);
pub const POLL: Duration = Duration::from_millis(20);
pub const CLEANUP_TIMEOUT: Duration = Duration::from_secs(5);
pub const HOST_CUSTODY_LOST: &str = "Host exited before custody";
pub const HUB_CUSTODY_LOST: &str = "Hub exited before custody";
pub const HUB_IDENTITY_PENDING: &str = "Hub identity not ready";
pub const REEXPORTS: u8 = 1;
pub const SESSION_NAME: &str = "remote bundle caller survival";
pub const CODEX_DIRECTORY: &str = "codex-source";
pub const CODEX_CONFIGURATION: &str = "config.toml";
pub const CODEX_CONFIGURATION_BYTES: &[u8] = b"model = 'fixture'\n";
pub const OUTPUT_COMMAND: &[u8] = b"printf 'YOLO_REMOTE_BUNDLE_%s\\n' SURVIVED\n";
pub const OUTPUT_MARKER: &str = "YOLO_REMOTE_BUNDLE_SURVIVED";

pub fn deadline() -> tokio::time::Instant {
    tokio::time::Instant::now() + TEST_TIMEOUT
}
