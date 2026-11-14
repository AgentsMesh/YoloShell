use std::time::Duration;

#[cfg(test)]
pub(super) const FIXTURE_BINARY_ENVIRONMENT: &str = "YOLO_TEST_FAILED_REARM_FIXTURE_BINARY";
pub(super) const PREPARE_MODE: &str = "prepare";
pub(super) const CRASH_MODE: &str = "crash";
pub(super) const RECOVER_MODE: &str = "recover";
pub(super) const DRAIN_STRESS_MODE: &str = "drain-stress";
pub(super) const CAPTURE_STRESS_MODE: &str = "capture-stress";
pub(super) const PROCESS_LIST_MODE: &str = "process-list";
#[cfg(test)]
pub(super) const FIXTURE_HOME_DIRECTORY: &str = "home";
#[cfg(not(test))]
pub(super) const PROCESS_LIST_EXECUTABLE: &str = "/bin/ps";
#[cfg(not(test))]
pub(super) const PROCESS_LIST_ARGUMENTS: &[&str] = &["-ww", "-axo", "pid=,command="];
#[cfg(test)]
pub(super) const CHILD_TIMEOUT: Duration = Duration::from_secs(30);
#[cfg(test)]
pub(super) const CHILD_POLL_INTERVAL: Duration = Duration::from_millis(10);
#[cfg(test)]
pub(super) const IDENTITY_CAPTURE_TIMEOUT: Duration = Duration::from_secs(2);
#[cfg(test)]
pub(super) const IDENTITY_CAPTURE_POLL_INTERVAL: Duration = Duration::from_millis(1);
#[cfg(not(test))]
pub(super) const CAPTURE_STRESS_DURATION: Duration = Duration::from_secs(30);
#[cfg(test)]
pub(super) const ADVERSARIAL_CHILD_TIMEOUT: Duration = Duration::from_millis(100);
pub(super) const KIBIBYTE_BYTES: usize = 1024;
pub(super) const DRAIN_STRESS_BYTES: usize = KIBIBYTE_BYTES * KIBIBYTE_BYTES;
#[cfg(not(test))]
pub(super) const DRAIN_STRESS_BYTE: u8 = b'x';
#[cfg(test)]
pub(super) const SUCCESS_EXIT_CODE: i32 = 0;
#[cfg(test)]
pub(super) const FIXTURE_BINARY_CONTEXT: &str = "resolve failed-resume rearm fixture binary";
#[cfg(not(test))]
pub(super) const FIXTURE_MODE_MISSING: &str = "failed-resume rearm fixture mode is missing";
#[cfg(not(test))]
pub(super) const FIXTURE_HOME_MISSING: &str = "failed-resume rearm fixture home is missing";
#[cfg(not(test))]
pub(super) const FIXTURE_BOUNDARY_MISSING: &str = "failed-resume rearm fixture boundary is missing";
#[cfg(not(test))]
pub(super) const FIXTURE_ARGUMENT_INVALID: &str =
    "failed-resume rearm fixture arguments are invalid";
pub(super) const FIXTURE_INPUT_CONTEXT: &str = "read private failed-resume rearm fixture input";
#[cfg(test)]
pub(super) const FIXTURE_CHILD_TIMEOUT: &str = "failed-resume rearm fixture child timed out";
#[cfg(test)]
pub(super) const FIXTURE_CHILD_STATUS: &str =
    "failed-resume rearm fixture child status is incorrect";
#[cfg(test)]
pub(super) const FIXTURE_CHILD_REAP: &str = "failed-resume rearm fixture child remains alive";
#[cfg(test)]
pub(super) const FIXTURE_OUTPUT_RESIDUE: &str = "failed-resume rearm fixture emitted output";
#[cfg(test)]
pub(super) const FIXTURE_OUTPUT_EMPTY: &str = "failed-resume rearm prepare output is empty";
#[cfg(test)]
pub(super) const FIXTURE_OUTPUT_ENCODING: &str = "failed-resume rearm fixture output is not UTF-8";
#[cfg(test)]
pub(super) const FIXTURE_DRAIN_FAILURE: &str = "failed-resume rearm fixture output drain failed";
#[cfg(test)]
pub(super) const FIXTURE_DRAIN_PANIC: &str = "failed-resume rearm fixture output drain panicked";
#[cfg(test)]
pub(super) const FIXTURE_CAPTURE_FAILURE: &str = "injected failed-resume child capture failure";
pub(super) const PROCESS_LIST_FAILURE: &str = "failed to inspect fixture process residue";
#[cfg(test)]
pub(super) const PROCESS_RESIDUE: &str = "failed-resume rearm fixture process remains";
#[cfg(test)]
pub(super) const TEMPORARY_RESIDUE: &str = "failed-resume rearm temporary root remains";
#[cfg(test)]
pub(super) const BOUNDARY_CONTEXT: &str = "exercise failed-resume rearm process boundary";
