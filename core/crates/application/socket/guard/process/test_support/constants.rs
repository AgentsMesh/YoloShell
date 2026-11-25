use std::time::Duration;

pub(super) const GUARD_BINARY_ENVIRONMENT: &str = "YOLO_SOCKET_ROOT_GUARD_BINARY";
pub(super) const FIXTURE_EXECUTABLE_ENVIRONMENT: &str =
    "YOLO_SOCKET_ROOT_GUARD_PROCESS_TEST_BINARY";
pub(super) const TEST_SOURCE_DIRECTORY_ENVIRONMENT: &str = "TEST_SRCDIR";
pub(super) const TEST_WORKSPACE_ENVIRONMENT: &str = "TEST_WORKSPACE";
pub(super) const TEST_EXECUTABLE_ENVIRONMENT: &str = "YOLO_TEST_EXECUTABLE";
pub(super) const TEST_FILTER_ENVIRONMENT: &str = "TESTBRIDGE_TEST_ONLY";
pub(super) const FIXTURE_MODE_ENVIRONMENT: &str = "YOLO_GUARD_PROCESS_FIXTURE_MODE";
pub(super) const FIXTURE_READY_ENVIRONMENT: &str = "YOLO_GUARD_PROCESS_FIXTURE_READY";
pub(super) const FIXTURE_RELEASE_ENVIRONMENT: &str = "YOLO_GUARD_PROCESS_FIXTURE_RELEASE";
pub(super) const FIXTURE_LEAF_IDENTITY_ENVIRONMENT: &str = "YOLO_GUARD_PROCESS_LEAF_IDENTITY";
pub(super) const FIXTURE_LEAF_READY_ENVIRONMENT: &str = "YOLO_GUARD_PROCESS_LEAF_READY";
pub(super) const FIXTURE_SOCKET_ROOT_ENVIRONMENT: &str = "YOLO_GUARD_PROCESS_SOCKET_ROOT";
pub(super) const FIXTURE_HELPER_ROLE_ENVIRONMENT: &str = "YOLO_GUARD_PROCESS_HELPER_ROLE";
pub(super) const FIXTURE_INTERMEDIATE_PID_ENVIRONMENT: &str = "YOLO_GUARD_PROCESS_INTERMEDIATE_PID";
pub(super) const FIXTURE_LATE_LEAF_IDENTITY_ENVIRONMENT: &str =
    "YOLO_GUARD_PROCESS_LATE_LEAF_IDENTITY";
pub(super) const FIXTURE_FORCE_SIGNAL_REQUEST_ENVIRONMENT: &str =
    "YOLO_GUARD_PROCESS_FORCE_SIGNAL_REQUEST";
pub(super) const FIXTURE_FORCE_SIGNAL_ACKNOWLEDGEMENT_ENVIRONMENT: &str =
    "YOLO_GUARD_PROCESS_FORCE_SIGNAL_ACKNOWLEDGEMENT";
pub(super) const FIXTURE_REFRESH_HANDOFF_REQUEST_ENVIRONMENT: &str =
    "YOLO_GUARD_PROCESS_REFRESH_HANDOFF_REQUEST";
pub(super) const FIXTURE_REFRESH_HANDOFF_ACKNOWLEDGEMENT_ENVIRONMENT: &str =
    "YOLO_GUARD_PROCESS_REFRESH_HANDOFF_ACKNOWLEDGEMENT";
pub(super) const FIXTURE_DISCOVERY_ARM_REQUEST_ENVIRONMENT: &str =
    "YOLO_GUARD_PROCESS_DISCOVERY_ARM_REQUEST";
pub(super) const FIXTURE_DISCOVERY_ARM_ACKNOWLEDGEMENT_ENVIRONMENT: &str =
    "YOLO_GUARD_PROCESS_DISCOVERY_ARM_ACKNOWLEDGEMENT";
pub(super) const FIXTURE_DISCOVERY_CAPABILITY_ENVIRONMENT: &str =
    "YOLO_GUARD_PROCESS_DISCOVERY_CAPABILITY";
pub(super) const FIXTURE_REFRESH_HOLDER_READY_ENVIRONMENT: &str =
    "YOLO_GUARD_PROCESS_REFRESH_HOLDER_READY";
pub(super) const FIXTURE_COORDINATOR_RETIRE_REQUEST_ENVIRONMENT: &str =
    "YOLO_GUARD_PROCESS_COORDINATOR_RETIRE_REQUEST";
pub(super) const FIXTURE_HISTORICAL_WITNESS_IDENTITY_ENVIRONMENT: &str =
    "YOLO_GUARD_PROCESS_HISTORICAL_WITNESS_IDENTITY";
pub(super) const FIXTURE_HISTORICAL_WITNESS_ACKNOWLEDGEMENT_ENVIRONMENT: &str =
    "YOLO_GUARD_PROCESS_HISTORICAL_WITNESS_ACKNOWLEDGEMENT";
pub(super) const NORMAL_MODE: &str = "normal";
pub(super) const RUNNING_LEAF_MODE: &str = "running-leaf";
pub(super) const STOPPED_LEAF_MODE: &str = "stopped-leaf";
pub(super) const FORK_ON_TERM_MODE: &str = "fork-on-term";
pub(super) const FORK_BEFORE_FORCE_MODE: &str = "fork-before-force";
pub(super) const FORK_AFTER_REFRESH_MODE: &str = "fork-after-refresh";
pub(super) const INTERMEDIATE_ROLE: &str = "intermediate";
pub(super) const LEAF_ROLE: &str = "leaf";
pub(super) const LATE_LEAF_ROLE: &str = "late-leaf";
pub(super) const REFRESH_HOLDER_ROLE: &str = "refresh-holder";
pub(super) const FIXTURE_TEST: &str = "fixture::guard_fixture";
pub(super) const ESCAPE_HELPER_TEST: &str = "fixture::escape_helper";
pub(super) const TEST_HARNESS_EXACT_ARGUMENT: &str = "--exact";
pub(super) const TEST_HARNESS_NOCAPTURE_ARGUMENT: &str = "--nocapture";
pub(super) const TEST_HARNESS_SINGLE_THREAD_ARGUMENT: &str = "--test-threads=1";
pub(super) const READY_FILE: &str = "fixture.ready";
pub(super) const RELEASE_FILE: &str = "fixture.release";
pub(super) const LEAF_IDENTITY_FILE: &str = "leaf.identity";
pub(super) const LEAF_READY_FILE: &str = "leaf.ready";
pub(super) const SOCKET_ROOT_FILE: &str = "socket-root";
pub(super) const FAILURE_TRIGGER_FILE: &str = "failure.trigger";
pub(super) const LATE_LEAF_IDENTITY_FILE: &str = "late-leaf.identity";
pub(super) const FORCE_SIGNAL_REQUEST_FILE: &str = "force-signal.request";
pub(super) const FORCE_SIGNAL_ACKNOWLEDGEMENT_FILE: &str = "force-signal.acknowledged";
pub(super) const REFRESH_HANDOFF_REQUEST_FILE: &str = "refresh-handoff.request";
pub(super) const REFRESH_HANDOFF_ACKNOWLEDGEMENT_FILE: &str = "refresh-handoff.acknowledged";
pub(super) const DISCOVERY_ARM_REQUEST_FILE: &str = "discovery-arm.request";
pub(super) const DISCOVERY_ARM_ACKNOWLEDGEMENT_FILE: &str = "discovery-arm.acknowledged";
pub(super) const REFRESH_HOLDER_READY_FILE: &str = "refresh-holder.ready";
pub(super) const REFRESH_POSTCHECK_WITNESS_FILE: &str = "refresh-postcheck.witness";
pub(super) const HISTORICAL_WITNESS_IDENTITY_FILE: &str = "historical-witness.identity";
pub(super) const HISTORICAL_WITNESS_ACKNOWLEDGEMENT_FILE: &str = "historical-witness.acknowledged";
pub(super) const COORDINATOR_RETIRE_REQUEST_FILE: &str = "coordinator-retire.request";
pub(super) const LOCAL_TEST_DEPENDENCY_DIRECTORY: &str = "deps";
pub(super) const LOCAL_GUARD_BINARY_FILE: &str = "yolo-socket-root-guard";
pub(super) const EMPTY_MARKER: &[u8] = b"";
pub(super) const POLL_INTERVAL: Duration = Duration::from_millis(10);
pub(super) const START_TIMEOUT: Duration = Duration::from_secs(8);
pub(super) const CLEANUP_TIMEOUT: Duration = Duration::from_secs(15);
pub(super) const FALLBACK_CLEANUP_TIMEOUT: Duration = Duration::from_secs(2);
pub(super) const FIXTURE_LIFETIME: Duration = Duration::from_secs(30);
pub(super) const SYSTEM_CALL_FAILURE: libc::pid_t = -1;
pub(super) const EXPECTED_SESSION_LEADERS: usize = 1;
pub(super) const IDENTITY_FIELD_SEPARATOR: char = ':';
pub(super) const FIXTURE_TIMEOUT_DETAIL: &str = "socket-root guard fixture timed out";
pub(super) const GUARD_EARLY_EXIT_DETAIL: &str =
    "socket-root guard exited before fixture readiness";
pub(super) const WATCHDOG_IDENTITY_DETAIL: &str = "detached watchdog identity is ambiguous";
pub(super) const CLEANUP_TIMEOUT_DETAIL: &str = "socket-root guard cleanup did not settle";
pub(super) const FIXTURE_MODE_DETAIL: &str = "socket-root guard fixture mode is invalid";
pub(super) const FIXTURE_PATH_DETAIL: &str = "socket-root guard fixture path is unavailable";
pub(super) const FIXTURE_SPAWN_DETAIL: &str = "spawn socket-root guard process fixture";
pub(super) const FIXTURE_LEAF_DETAIL: &str = "capture escaped fixture leaf";
pub(super) const STOPPED_LEAF_DETAIL: &str = "escaped fixture leaf did not stop stably";
pub(super) const GUARD_BINARY_DETAIL: &str = "socket-root guard binary is unavailable";
pub(super) const GUARD_KILL_DETAIL: &str = "socket-root guard exited before exact kill delivery";
pub(super) const GUARD_STATUS_DETAIL: &str = "socket-root guard exit status is unexpected";
