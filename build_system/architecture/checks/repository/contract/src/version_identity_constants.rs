pub(super) const VERSION_PATH: &str = "build_system/bazel/version.bzl";
pub(super) const SNAPSHOT_PATH: &str = "build_system/release/contracts/src/snapshot.rs";
pub(super) const LINUX_PATH: &str = "build_system/release/contracts/src/linux.rs";
pub(super) const VERSION_NAME: &str = "YOLO_MARKETING_VERSION";
pub(super) const RUST_DECLARATION: &str = "pub const YOLO_MARKETING_VERSION: &str = \"";
pub(super) const LINUX_DECLARATION: &str = "pub const LINUX_RUNTIME_VERSION: &str = \"";
pub(super) const MISSING_MESSAGE: &str =
    "marketing version must be declared in version.bzl and the release snapshot";
pub(super) const MISMATCH_MESSAGE: &str = "Bazel and release marketing versions must match";
pub(super) const DUPLICATE_MESSAGE: &str =
    "release contracts must not redeclare the marketing version";
