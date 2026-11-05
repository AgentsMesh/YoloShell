pub const CONFIG_PATH: &str = "/tmp/yolo-broker-supervision-config";
pub const STATE_PATH: &str = "/tmp/yolo-broker-supervision-state";
pub const CONFIG_REVISION: &str =
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
pub const STATE_ANCHOR: &str = "1:2";
pub const HOST_ID: &str = "state-test-host";
pub const START_TIMEOUT: std::time::Duration = yolo_remote_broker::BROKER_READY_TIMEOUT;
pub const BOOT_ID_FIELD: &str = "boot_id";
pub const UNEXPECTED_FIELD: &str = "unexpected";
pub const TEST_HARNESS_EXACT_ARGUMENT: &str = "--exact";
pub const TEST_HARNESS_NOCAPTURE_ARGUMENT: &str = "--nocapture";
pub const TEST_EXIT_EXECUTABLE: &str = yolo_application_test_fixtures::executables::TRUE;
pub use yolo_cli::constants::BROKER_SUPERVISION_AUTHORITY_ENVIRONMENT;
