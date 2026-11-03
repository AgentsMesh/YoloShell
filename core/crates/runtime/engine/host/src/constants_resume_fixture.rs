#[cfg(any(test, feature = "failed-rearm-test-hooks"))]
pub(in crate::host) const RESUME_TEST_MISSING_RUNTIME: &str = "missing-resume-runtime";
#[cfg(test)]
pub(in crate::host) const RESUME_TEST_SLEEP_EXECUTABLE: &str = "/bin/sleep";
#[cfg(test)]
pub(in crate::host) const RESUME_TEST_SLEEP_ARGUMENT: &str = "30";
#[cfg(test)]
pub(in crate::host) const RESUME_TEST_BOOT_ID: &str = "resume-test-boot";
#[cfg(any(test, feature = "failed-rearm-test-hooks"))]
pub(in crate::host) const RESUME_TEST_PREVIOUS_BOOT_ID: &str = "resume-test-previous-boot";
#[cfg(test)]
pub(in crate::host) const RESUME_TEST_GENERATION_OFFSET: u64 = 7;
#[cfg(any(test, feature = "failed-rearm-test-hooks"))]
pub(in crate::host) const RESUME_TEST_PROCESS_START_TIME: u64 = 1;
#[cfg(any(test, feature = "failed-rearm-test-hooks"))]
pub(in crate::host) const RESUME_TEST_DEAD_WORKER_PID: u32 = i32::MAX as u32;
#[cfg(any(test, feature = "failed-rearm-test-hooks"))]
pub(in crate::host) const RESUME_TEST_DEAD_CHILD_PID: u32 = i32::MAX as u32 - 1;
