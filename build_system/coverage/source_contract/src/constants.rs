pub const CURRENT_DIRECTORY_PREFIX: &str = "./";
pub const PROCESS_CWD_PREFIX: &str = "#/proc/self/cwd/";
pub(crate) const DECLARATION_SOURCE_EXTENSION: &str = "rs";
pub(crate) const DECLARATION_BAZEL_OUTPUT: &str = "bazel-out";
pub(crate) const DECLARATION_BAZEL_BINARY: &str = "bin";
pub(crate) const DECLARATION_TEST_DIRECTORIES: &[&str] = &["unit_tests", "integration_tests"];
pub(crate) const REPOSITORY_TEST_ROOT: &str = "integration_tests/";
pub(crate) const INVALID_REPOSITORY_TEST_PATH: &str = "invalid repository test source path";
pub(crate) const TEST_SUPPORT_DIRECTORY: &str = "test_support";
pub(crate) const EXTERNAL_ARTIFACT_DIRECTORY: &str = "external";
pub(crate) const COVERAGE_TOOL_ARTIFACT_SUFFIXES: &[&str] = &[
    "/llvm/bin/llvm-cov",
    "/llvm/bin/llvm-profdata",
    "/llvm/lib/libLTO.dylib",
    "/llvm/lib/libRemarks.dylib",
    "/llvm/lib/libLLVM.so",
    "/llvm/lib/LLVM.dll",
];
