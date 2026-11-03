pub const TEST_LIST_ARGUMENT: &str = "--list";
pub const BASH_EXECUTABLE: &str = "/bin/bash";
pub const SLEEP_EXECUTABLE: &str = "/bin/sleep";
pub const YES_EXECUTABLE: &str = "/usr/bin/yes";
pub const SHELL_COMMAND_ARGUMENT: &str = "-c";
pub const LONG_SLEEP_SECONDS: &str = "5";
pub const UNRELATED_PROCESS_SLEEP_SECONDS: &str = "120";
pub const PARENT_EXIT_SCRIPT: &str = "sleep 5 & exit 0";
pub const TERM_IGNORING_DESCENDANT_SCRIPT: &str = "(trap '' TERM; sleep 5) & exit 0";
pub const LARGE_DUAL_OUTPUT_SCRIPT: &str = "yes stdout & yes stderr >&2 & wait";
pub const OUTPUT_LIMIT_BYTES: usize = 4 * 1024;
pub const TEST_TIMEOUT_MILLISECONDS: u64 = 25;
pub const SUCCESS_TIMEOUT_SECONDS: u64 = 1;
pub const PROCESS_COMPLETION_LIMIT_SECONDS: u64 = 4;
pub const PROCESS_CLEANUP_OBSERVATION_LIMIT_SECONDS: u64 = 8;
pub const ESCAPED_DESCENDANT_SECONDS: u32 = 3;
pub const TEST_EXACT_ARGUMENT: &str = "--exact";
pub const TEST_IGNORED_ARGUMENT: &str = "--ignored";
pub const ESCAPED_DESCENDANT_FIXTURE: &str =
    "integration_tests::process_escape_boundary_tests::escaped_pipe_holder_fixture";
pub const ESCAPED_CLOSED_DESCENDANT_FIXTURE: &str =
    "integration_tests::process_escape_boundary_tests::escaped_closed_pipe_fixture";
pub const DOUBLE_FORK_ESCAPE_FIXTURE: &str =
    "integration_tests::process_escape_boundary_tests::immediate_double_fork_fixture";
pub const CONTENDED_DOUBLE_FORK_ESCAPE_FIXTURE: &str =
    "integration_tests::process_escape_boundary_tests::term_resistant_double_fork_fixture";
pub const ESCAPED_PID_DESCRIPTOR: libc::c_int = 518;
pub const ESCAPED_PIPE_HOLDER_DESCRIPTION: &str = "escaped pipe holder";
pub const ESCAPED_CLOSED_PIPE_DESCRIPTION: &str = "escaped closed-pipe descendant";
pub const DOUBLE_FORK_ESCAPE_DESCRIPTION: &str = "immediate double-fork escape";
pub const CONTENDED_DOUBLE_FORK_ESCAPE_DESCRIPTION: &str =
    "term-resistant concurrent double-fork escape";
pub const ESCAPE_STRESS_CASES: usize = 8;
pub const ESCAPE_CONTENTION_CASES: usize = 16;
pub const PRIVATE_DIRECTORY_MODE: u32 = 0o700;
