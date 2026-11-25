use std::time::Duration;

pub const PINNED_BAZEL_VERSION: &str = "bazel 8.2.1";
pub const PINNED_BAZEL_SHA256: &str =
    "22ff65b05869f6160e5157b1b425a14a62085d71d8baef571f462b8fe5a703a3";
pub const PINNED_BAZEL_CACHE_PATH: &str =
    "Library/Caches/bazelisk/downloads/bazelbuild/bazel-8.2.1-darwin-arm64/bin/bazel";
pub const BAZEL_VERSION_ARGUMENT: &str = "--version";
pub const BAZEL_LAUNCHER_PREFIX: &str = "yolo-pinned-bazel-";
pub const BAZEL_LAUNCHER_NAME: &str = "bazel";
pub const BAZEL_BATCH_ARGUMENT: &str = "--batch";
pub const BAZEL_BUILD_VERB: &str = "build";
pub const BAZEL_CQUERY_VERB: &str = "cquery";
pub const BAZEL_QUERY_STARLARK_OUTPUT_ARGUMENT: &str = "--output=starlark";
pub const BAZEL_QUERY_EXECUTABLE_EXPRESSION_ARGUMENT: &str =
    "--starlark:expr=providers(target)[\"DefaultInfo\"].files_to_run.executable.path";
pub const BAZEL_EXECROOT_OUTPUT_PREFIX: &str = "bazel-out/";
pub const BAZEL_OUTPUT_LINK_SUFFIX: &str = "out/";
pub const BAZEL_OPTIMIZED_ARGUMENT: &str = "--compilation_mode=opt";
pub const BAZEL_TEST_VERB: &str = "test";
pub const BAZEL_COVERAGE_VERB: &str = "coverage";
pub const BAZEL_RUN_VERB: &str = "run";
pub const BAZEL_JOBS_ARGUMENT_PREFIX: &str = "--jobs=";
pub const SERIAL_JOBS_ARGUMENT: &str = "--jobs=1";
pub const BAZEL_RUST_CONFIG_ARGUMENT: &str = "--config=rust";
pub const BAZEL_MACOS_CONFIG_ARGUMENT: &str = "--config=macos";
pub const BAZEL_ARGUMENT_SEPARATOR: &str = "--";
pub const BAZEL_NO_SYSTEM_RC_ARGUMENT: &str = "--nosystem_rc";
pub const BAZEL_NO_HOME_RC_ARGUMENT: &str = "--nohome_rc";
pub const BAZEL_NO_WORKSPACE_RC_ARGUMENT: &str = "--noworkspace_rc";
pub const BAZEL_TRACKED_RC_ARGUMENT: &str = "--bazelrc=.bazelrc";
pub const BAZEL_OUTPUT_ROOT_PREFIX: &str = "--output_user_root=";
pub const BAZEL_SYMLINK_PREFIX: &str = "--symlink_prefix=";
pub const BAZEL_STARTUP_ARGUMENTS: &[&str] = &[
    BAZEL_BATCH_ARGUMENT,
    BAZEL_NO_SYSTEM_RC_ARGUMENT,
    BAZEL_NO_HOME_RC_ARGUMENT,
    BAZEL_NO_WORKSPACE_RC_ARGUMENT,
    BAZEL_TRACKED_RC_ARGUMENT,
];
pub const BAZEL_VERSION_TIMEOUT: Duration = Duration::from_secs(10);
pub const AUDIT_SCRIPT_DESCRIPTOR: libc::c_int = 512;
pub const AUDIT_ARTIFACT_DESCRIPTOR: libc::c_int = 513;
pub const AUDIT_BUNDLE_POLICY_DESCRIPTOR: libc::c_int = 514;
pub const AUDIT_ARCHIVE_POLICY_DESCRIPTOR: libc::c_int = 515;
pub const AUDIT_ICON_DESCRIPTOR: libc::c_int = 516;
pub const AUDIT_DEPENDENCY_POLICY_DESCRIPTOR: libc::c_int = 517;
pub const AUDIT_LINUX_MANIFEST_DESCRIPTOR: libc::c_int = 519;
pub const AUDIT_ARTIFACT_COPY_DESCRIPTOR: libc::c_int = 520;
pub const AUDIT_ARTIFACT_AFTER_DESCRIPTOR: libc::c_int = 521;
pub const AUDIT_ARTIFACT_VERIFY_DESCRIPTOR: libc::c_int = 522;
pub const AUDIT_LINUX_MANIFEST_VERIFY_DESCRIPTOR: libc::c_int = 523;
