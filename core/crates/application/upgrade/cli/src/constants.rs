pub(crate) const COMMAND: &str = "update";
pub(crate) const PROGRAM: &str = "yolo";
pub(crate) const SERVICE_UNIT_ARGUMENT: &str = "service-unit";
pub(crate) const SOCKET_ARGUMENT: &str = "socket";
pub(crate) const HUB_ARGUMENT: &str = "hub";
pub(crate) const STATE_ARGUMENT: &str = "state";
pub(crate) const JSON_ARGUMENT: &str = "json";
pub(crate) const HELP_ARGUMENT: &str = "--help";
pub(crate) const SERVICE_UNIT_IMAGE_DIGEST: &str = "X-Yolo-Image-SHA256=";
pub(crate) const SERVICE_UNIT_IMAGE_SIZE: &str = "X-Yolo-Image-Size=";
pub(crate) const EXEC_START: &str = "ExecStart=";
pub(crate) const IMAGE_NAME: &str = "yolo";
pub(crate) const CHECK_DEADLINE_SECONDS: u64 = 5;
pub(crate) const CHECK_CANDIDATE: &str = "candidate_image";
pub(crate) const CHECK_PROTOCOL: &str = "hub_protocol";
pub(crate) const CHECK_BUILD: &str = "live_service_images";
pub(crate) const CHECK_STATE: &str = "state_contract";
pub(crate) const CHECK_SERVICE_PREFIX: &str = "service_unit:";
pub(crate) const READY: &str = "ready";
pub(crate) const REFUSED: &str = "refused";
pub(crate) const HOST_RUNNER_UNIT: &str = "/etc/systemd/system/yolo-host-runner.service";
pub(crate) const HUB_UNIT: &str = "/etc/systemd/system/yolo-hub.service";
pub(crate) const CHECK_SERVICE_UNITS: &str = "service_units";
pub(crate) const EMPTY_EXEC_START: &str = "service unit ExecStart is empty";
pub(crate) const ABSOLUTE_EXEC_START: &str = "service ExecStart must be absolute";
pub(crate) const WRONG_EXECUTABLE: &str = "service ExecStart must point to yolo";
pub(crate) const MUTABLE_EXECUTABLE: &str = "service ExecStart must point to an immutable image";
pub(crate) const UPGRADE_REFUSED: &str =
    "upgrade admission refused; run `yolo restart` to activate the installed image";
pub(crate) const RESTART_CHECK_CONFLICT: &str = "--restart cannot be combined with --check-release";
pub(crate) const DIGEST_MISMATCH: &str = "service image digest does not match executable";
pub(crate) const SIZE_MISMATCH: &str = "service image size does not match executable";
pub(crate) const CONTRACT_MATCH: &str = "runtime contract matches current image";
pub(crate) const NO_SERVICE_UNITS: &str = "no managed systemd service units were discovered";
pub(crate) const CHECK_RELEASE_ARGUMENT: &str = "check-release";
pub(crate) const INSTALL_RELEASE_ARGUMENT: &str = "install-release";
pub(crate) const NIGHTLY_VALUE: &str = "nightly";
pub(crate) const CURL_HEADER: &str = "--header";
pub(crate) const CURL_OUTPUT: &str = "--output";
pub(crate) const CURL_PROTO: &str = "--proto";
pub(crate) const CURL_PROTO_REDIRECT: &str = "--proto-redir";
pub(crate) const CURL_TLS: &str = "--tlsv1.2";
pub(crate) const CURL_LOCATION: &str = "--location";
pub(crate) const CURL_FAIL: &str = "--fail";
pub(crate) const CURL_SILENT: &str = "--silent";
pub(crate) const CURL_SHOW_ERROR: &str = "--show-error";
pub(crate) const TARGET_MACOS_ARM64: &str = "aarch64-apple-darwin";
pub(crate) const TARGET_LINUX_X86_64: &str = "x86_64-unknown-linux-gnu";
pub(crate) const TARGET_LINUX_ARM64: &str = "aarch64-unknown-linux-gnu";
pub(crate) const CHECK_RELEASE_SWITCH: &str = "--check-release";
pub(crate) const RESTART_SWITCH: &str = "--restart";
pub(crate) const CHANNEL_SWITCH: &str = "--channel";

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub(crate) const TARGET_TRIPLE: &str = TARGET_MACOS_ARM64;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) const TARGET_TRIPLE: &str = TARGET_LINUX_X86_64;
#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
pub(crate) const TARGET_TRIPLE: &str = TARGET_LINUX_ARM64;
#[cfg(not(any(
    all(target_os = "macos", target_arch = "aarch64"),
    all(target_os = "linux", target_arch = "x86_64"),
    all(target_os = "linux", target_arch = "aarch64")
)))]
pub(crate) const TARGET_TRIPLE: &str = "unsupported";
pub(crate) const GITHUB_ACCEPT: &str = "Accept: application/vnd.github+json";
