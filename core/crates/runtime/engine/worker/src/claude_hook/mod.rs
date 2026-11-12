mod admission_pump;
mod admission_pump_control;
pub(in crate::worker) mod binding;
mod constants;
mod control_source;
mod handler;
mod native_admission;
mod native_environment;
mod native_evidence;
mod native_executable;
pub(in crate::worker) mod native_runtime;
mod native_transaction;
mod peer;
mod permission;
mod server;

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../test_support/unit/claude_hook/test_session_manifest.rs"]
mod test_session_manifest;

pub(super) use admission_pump::spawn_admission_pump;
#[cfg(test)]
pub(super) use admission_pump::spawn_admission_pump_after_test_barrier;
pub(super) use admission_pump_control::ClaudeHookAdmissionPump;
pub(in crate::worker) use control_source::with_control_source;
pub(super) use native_environment::ClaudeHookEnvironment;
pub(super) use server::{ClaudeHookServer, handle_accept_result, handle_listener_failure};

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../unit_tests/claude_hook/credential_response_tests.rs"]
mod credential_response_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../unit_tests/claude_hook/environment_server_tests.rs"]
mod environment_server_tests;
