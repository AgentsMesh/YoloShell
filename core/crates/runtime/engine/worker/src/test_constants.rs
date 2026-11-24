pub(in crate::worker) const BOOTSTRAP_TEST_EXACT_ARGUMENT: &str = "--exact";
pub(in crate::worker) const BOOTSTRAP_TEST_NOCAPTURE_ARGUMENT: &str = "--nocapture";
pub(in crate::worker) const BOOTSTRAP_TEST_FAILURE_EXIT_CODE: i32 = 17;
#[cfg(feature = "yolo-unit-tests")]
pub const BOOTSTRAP_TEST_ENTRY: &str =
    "worker::bootstrap::bootstrap_gate::child_entrypoint::bootstrap_gate_child_process";
#[cfg(feature = "yolo-integration-tests")]
pub const BOOTSTRAP_TEST_ENTRY: &str =
    "worker::bootstrap::bootstrap_gate::integration_process_helper::bootstrap_gate_child_process_integration";
pub(in crate::worker) const BOOTSTRAP_TEST_PRE_EXEC_MARKER_ENV: &str =
    "YOLO_BOOTSTRAP_TEST_PRE_EXEC_MARKER";
pub(in crate::worker) const BOOTSTRAP_TEST_PRE_EXEC_MARKER_CONTENT: &[u8] = b"ready";
pub(in crate::worker) const BOOTSTRAP_TEST_PRE_EXEC_MARKER_WRITE_CONTEXT: &str =
    "write bootstrap pre-exec test marker";
pub(in crate::worker) const CONTROLLER_PERSIST_FAILURE_TEST_DETAIL: &str =
    "injected controller generation persistence failure";
