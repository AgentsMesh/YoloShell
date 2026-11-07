pub(crate) const ROLE_ENV: &str = "YOLO_BASH_FIXTURE_CLEANUP_ROLE";
pub(crate) const EVIDENCE_ENV: &str = "YOLO_BASH_FIXTURE_CLEANUP_EVIDENCE";
pub(crate) const READER_ERROR_ROLE: &str = "reader-construction-error";
pub(crate) const UNWIND_ROLE: &str = "early-unwind";
pub(crate) const HELPER_TEST: &str =
    "worker::tests::bash_fixture_cleanup_tests::bash_fixture_cleanup_helper";
pub(crate) const EVIDENCE_FILE: &str = "bash-child.json";
pub(crate) const INJECTED_UNWIND: &str = "injected failure before BashPty construction";
pub(crate) const PROMPT_TIMEOUT: &str = "ordinary Bash fixture did not display its prompt";
pub(crate) const MISSING_MASTER: &str = "ordinary Bash fixture has no master descriptor";
pub(crate) const SYSCALL_FAILURE: i32 = -1;
