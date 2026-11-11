#[cfg(test)]
pub use yolo_architecture_source_policy::constants::*;
pub const ROOT_OVERRIDE_ENV: &str = "YOLO_ARCH_ROOT";
pub const CONFORMANCE_ROOT_ENV: &str = "YOLO_ARCH_CONFORMANCE_ROOT";
pub const TEST_SOURCE_DIRECTORY_ENV: &str = "TEST_SRCDIR";
pub const TEST_WORKSPACE_ENV: &str = "TEST_WORKSPACE";
pub const RUNFILES_DIRECTORY_ENV: &str = "RUNFILES_DIR";
pub const RUNFILES_MANIFEST_ENV: &str = "RUNFILES_MANIFEST_FILE";
pub const BUILD_WORKSPACE_ENV: &str = "BUILD_WORKSPACE_DIRECTORY";
pub const BAZEL_MAIN_REPOSITORY_DIRECTORY: &str = "_main";
pub const BAZEL_RUNFILES_DIRECTORY_SUFFIX: &str = ".runfiles";
pub const WORKSPACE_MARKER: &str = "MODULE.bazel";
pub const TEXT_EXTENSIONS: &[&str] = &[
    "bash",
    "bazel",
    "bzl",
    "capnp",
    "cjs",
    "css",
    "exp",
    "h",
    "html",
    "js",
    "json",
    "jsonl",
    "md",
    "mjs",
    "modulemap",
    "patch",
    "plist",
    "rs",
    "sh",
    "toml",
    "ts",
    "tsx",
    "yaml",
    "yml",
    "zsh",
];
pub const TEXT_FILE_NAMES: &[&str] = &[
    ".bazelignore",
    ".bazelrc",
    ".bazelversion",
    ".gitattributes",
    ".gitignore",
    "BUILD",
    "Cargo.lock",
    "Package.resolved",
];
pub const IGNORED_ROOT_DIRECTORIES: &[&str] = &[
    ".build",
    ".git",
    ".verification",
    ".yolo-dev",
    "bazel-bin",
    "bazel-out",
    "bazel-testlogs",
    "coverage",
    "dist",
    "target",
    "test-results",
];
#[cfg(test)]
pub use yolo_architecture_foundation::text::joined;
