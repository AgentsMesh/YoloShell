pub const COVERAGE_DIRECTORY_ENV: &str = "COVERAGE_DIR";
pub const EXECUTION_ROOT_ENV: &str = "ROOT";
pub const RUNFILES_DIRECTORY_ENV: &str = "RUNFILES_DIR";
pub const TEST_WORKSPACE_ENV: &str = "TEST_WORKSPACE";
pub const TEST_BINARY_ENV: &str = "TEST_BINARY";
pub const INSTRUMENTED_TEST_BINARY_ENV: &str = "YOLO_TEST_EXECUTABLE";
pub const ADDITIONAL_COVERAGE_OBJECTS_ENV: &str = "YOLO_ADDITIONAL_COVERAGE_OBJECTS";
pub const LLVM_COV_ENV: &str = "RUST_LLVM_COV";
pub const LLVM_PROFDATA_ENV: &str = "RUST_LLVM_PROFDATA";
pub const VERBOSE_ENV: &str = "VERBOSE_COVERAGE";
pub(crate) const COVERAGE_MANIFEST_ENV: &str = "COVERAGE_MANIFEST";
pub(crate) const DECLARATION_FILE_LIMIT: u64 = 128 * 1024;
pub(crate) const TEST_ONLY_DETAIL: &str =
    "verified executed test-only sources have no production coverage denominator";
pub(crate) const DECLARATION_ONLY_DETAIL: &str =
    "verified declaration-only production sources have no LLVM coverage mappings";
pub(crate) const PREMATURE_EXIT_ENV: &str = "TEST_PREMATURE_EXIT_FILE";
pub(crate) const COMPLETION_MODE: u32 = 0o600;
pub(crate) const COMPLETION_LINKS: u64 = 1;
pub(crate) const INCOMPLETE_COVERAGE: &str = "Rust coverage collection did not complete\n";
pub(crate) const COMPLETION_CHANGED: &str = "coverage completion marker changed";

pub const COVERAGE_REPORT_NAME: &str = "coverage.dat";
pub const PROFILE_DATA_NAME: &str = "coverage.profdata";
pub const RAW_PROFILE_EXTENSION: &str = "profraw";
pub const RUNFILES_CONFIGURATION_COMPONENTS: usize = 3;
pub const FAILURE_EXIT_CODE: i32 = 1;
pub const PRIMARY_INSTRUMENTED_OBJECT_COUNT: usize = 1;

pub const MERGE_COMMAND: &str = "merge";
pub const SPARSE_ARGUMENT: &str = "--sparse";
pub const OUTPUT_ARGUMENT: &str = "--output";
pub const EXPORT_COMMAND: &str = "export";
pub const REPORT_COMMAND: &str = "report";
pub const LCOV_FORMAT_ARGUMENT: &str = "-format=lcov";
pub const JSON_FORMAT_ARGUMENT: &str = "-format=text";
pub const PROFILE_ARGUMENT: &str = "-instr-profile";
pub const OBJECT_ARGUMENT: &str = "-object";
pub const EXTERNAL_IGNORE_ARGUMENT: &str = "-ignore-filename-regex=.*external/.+";
pub const TEMPORARY_IGNORE_ARGUMENT: &str = "-ignore-filename-regex=/tmp/.+";
pub const PATH_EQUIVALENCE_PREFIX: &str = "-path-equivalence=.,";
pub const SHOW_FUNCTIONS_ARGUMENT: &str = "-show-functions";
pub const SHOW_BRANCH_SUMMARY_ARGUMENT: &str = "-show-branch-summary";
pub const SHOW_REGION_SUMMARY_ARGUMENT: &str = "-show-region-summary";
pub const SOURCES_ARGUMENT: &str = "--sources";
#[cfg(test)]
pub const BAZEL_COVERAGE_DIRECTORY_ARGUMENT: &str = "--coverage_dir=";
#[cfg(test)]
pub const BAZEL_OUTPUT_FILE_ARGUMENT: &str = "--output_file=";
#[cfg(test)]
pub const BAZEL_SOURCE_MANIFEST_ARGUMENT: &str = "--source_file_manifest=";

#[cfg(test)]
pub const FUNCTION_REPORT_DIVIDER_SAMPLE: &str = "--------------------";
#[cfg(test)]
pub const FUNCTION_REPORT_SHORT_DIVIDER_SAMPLE: &str = "--";
