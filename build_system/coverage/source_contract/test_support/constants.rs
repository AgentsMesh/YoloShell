pub const SOURCE: &str = "build_system/fixture/src/lib.rs";
pub const TEST_SOURCE: &str = "build_system/fixture/unit_tests/case.rs";
pub const TEST_SUPPORT_SOURCE: &str = "build_system/fixture/test_support/parser.rs";
pub const BAZEL_WITNESS_PREFIX: &str = "bazel-out/test-config/bin/";
pub const FIRST_SOURCE: usize = 0;
pub const EXTENSIONLESS_INTEGRATION_DIRECTORY: &str = "integration_tests";
pub const EXTRA_DEFINITION: &[u8] = b"\npub fn unexpected_runtime_code() {}\n";
pub const REPOSITORY_TEST_ROOTS: &[&str] = &[
    "core/src/",
    "core/crates/fixture/",
    "plugins/tui/fixture/",
    "plugins/web/gateway/",
    "build_system/fixture/",
    "integration_tests/fixture/",
];
pub const REPOSITORY_UNIT_SOURCE: &str = "integration_tests/runtime/fixture/unit_tests/case.rs";
pub const REPOSITORY_FIXTURE_SOURCE: &str = "integration_tests/runtime/fixture/parser.rs";
pub const TEST_ONLY_ARTIFACTS: &[&str] = &[
    "bazel-out/test-config/bin/build_system/fixture/unit_tests",
    "external/toolchain/llvm/bin/llvm-cov",
    "external/toolchain/llvm/bin/llvm-profdata",
    "external/toolchain/llvm/lib/libLTO.dylib",
    "external/toolchain/llvm/lib/libRemarks.dylib",
    "external/toolchain/llvm/lib/libLLVM.so",
    "external/toolchain/llvm/lib/LLVM.dll",
];
