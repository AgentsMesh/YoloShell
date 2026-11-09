pub const FUNCTION: &str = "fixture_function";
pub const OTHER_FUNCTION: &str = "other_function";
pub const BAZEL_WITNESS_PREFIX: &str = "bazel-out/test-config/bin/";
pub const TEST_WORKSPACE: &str = "_main";
pub const FIXTURE_TOOL_ENV: &str = "YOLO_COLLECTOR_LLVM_FIXTURE";
pub const FIXTURE_DIRECTORY_ENV: &str = "YOLO_COLLECTOR_FIXTURE_DIRECTORY";
pub const FIXTURE_FAILURE_ENV: &str = "YOLO_COLLECTOR_FIXTURE_FAILURE";
pub const FIXTURE_CHILD_ENV: &str = "YOLO_COLLECTOR_FIXTURE_CHILD";
pub const CHILD_TEST: &str = "entrypoint_tests::collector_child";
pub const FIXTURE_TOOL: &str = "tools/llvm-fixture";
pub const FIXTURE_BINARY: &str = "tests/instrumented";
pub const FIXTURE_ADDITIONAL: &str = "tests/additional";
pub const FIXTURE_RAW: &str = "input.profraw";
pub const FIXTURE_COVERAGE: &str = "coverage";
pub const FIXTURE_MARKER: &str = "premature-exit";
pub const FIXTURE_RUNFILES: &str = "bazel-out/test-config/bin/collector.runfiles";
pub const FIXTURE_SOURCE: &str = "build_system/fixture/src/lib.rs";
pub const TEST_SUPPORT_SOURCE: &str = "build_system/fixture/test_support/parser.rs";
pub const OTHER_TEST_SOURCE: &str = "build_system/fixture/unit_tests/other.rs";
pub const REPOSITORY_TEST_SOURCE: &str = "integration_tests/runtime/fixture/unit_tests/case.rs";
pub const REPOSITORY_SUPPORT_SOURCE: &str = "integration_tests/runtime/fixture/parser.rs";
pub const OTHER_REPOSITORY_TEST_SOURCE: &str =
    "integration_tests/runtime/fixture/unit_tests/other.rs";
pub const LCOV_STAGE: &str = "lcov";
pub const JSON_STAGE: &str = "json";
pub const REPORT_STAGE: &str = "report";
pub const MERGE_STAGE: &str = "merge";
pub const UTF8_FAILURE: &str = "utf8";
pub const CLEANUP_FAILURE: &str = "cleanup";
pub const MARKER_FAILURE: &str = "marker";
pub const ENTRY_READ_FAILURE: &str = "entry-read";
pub const ENTRY_READ_ERROR: &str = "injected directory-entry read failure";
pub const ENUMERATION_CONTEXT: &str = "cannot enumerate raw profiles in";
#[cfg(not(test))]
pub const TOOL_FAILURE_CODE: i32 = 19;
#[cfg(not(test))]
pub const FIXTURE_FAILURE_MESSAGE: &str = "fixture stage failed";
pub const LCOV_FILE: &str = "export.lcov";
pub const JSON_FILE: &str = "export.json";
pub const REPORT_FILE: &str = "functions.txt";
pub const EXACT_TEST_ARGUMENT: &str = "--exact";
pub const NO_CAPTURE_ARGUMENT: &str = "--nocapture";
#[cfg(not(test))]
pub const FIXTURE_OUTPUT_ARGUMENT: &str = "--output";
#[cfg(not(test))]
pub const FIXTURE_EXPORT_COMMAND: &str = "export";
pub const SATURATED_EXECUTION_PATHS: &[&str] = &[
    "/data/0/functions/0/count",
    "/data/0/functions/0/regions/0/4",
    "/data/0/functions/0/branches/0/4",
    "/data/0/functions/0/branches/0/5",
];
