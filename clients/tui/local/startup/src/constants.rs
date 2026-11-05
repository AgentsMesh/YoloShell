pub(crate) const BOOTSTRAP_ARGUMENT: &str = "--bootstrap";
pub(crate) const HOST_EXECUTABLE_ARGUMENT: &str = "--host-executable";
pub(crate) const EXECUTABLE_MASK: u32 = 0o111;
pub(crate) const WRITABLE_BY_OTHERS_MASK: u32 = 0o022;
pub(crate) const OVERFLOW_BYTE: u64 = 1;
pub(crate) const PACKAGE_INVALID: &str =
    "local Hub requires an executable Yolo program that is not writable by others";
pub(crate) const RECEIPT_INVALID: &str =
    "local Hub bootstrap receipt does not match the selected profile";
pub(crate) const OUTPUT_MISSING: &str = "local Hub bootstrap output pipe is unavailable";
pub(crate) const OUTPUT_TOO_LARGE: &str = "local Hub bootstrap receipt exceeds its byte limit";
pub(crate) const TIMED_OUT: &str = "local Hub bootstrap deadline expired";
pub(crate) const START_FAILED: &str = "local Yolo services failed to start; see the message above";
pub(crate) const CANCELLED: &str = "local Hub startup was cancelled";
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub(crate) const AUTOMATIC_UNSUPPORTED: &str =
    "automatic local Hub startup requires macOS or Linux; supply --hub and --socket or --endpoint";
