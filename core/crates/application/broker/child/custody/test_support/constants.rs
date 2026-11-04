pub use yolo_remote_broker::BROKER_READY_TIMEOUT as START_TIMEOUT;
pub const STATE_DIRECTORY: &str = "state";
pub const TEST_SLEEP_EXECUTABLE: &str = "/bin/sleep";
pub const TEST_SLEEP_DURATION: &str = "10";
pub const TEST_EXIT_EXECUTABLE: &str = crate::test_executables::TRUE;
pub const BROKER_REAP_WAIT_ATTEMPTS: usize = 200;
pub const BROKER_REAP_WAIT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(5);
pub const TEST_REAPER_HANDOFF_FAILURE: &str = "test reaper handoff failure";
pub const TEST_POLL_OPERATION: &str = "poll";
pub const TEST_TERMINATE_OPERATION: &str = "terminate";
pub const EXIT_BARRIER_HELPER_FILTER: &str =
    "exit_barrier::termination_exit_barrier_subprocess_helper";
pub const TEST_HARNESS_NOCAPTURE_ARGUMENT: &str = "--nocapture";
pub const TEST_HARNESS_EXACT_ARGUMENT: &str = "--exact";
