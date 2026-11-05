pub(crate) const SCHEMA_VERSION: u16 = 1;
pub(crate) const APPLICATION_DIRECTORY: &str = "Yolo";
pub(crate) const STATE_DIRECTORY: &str = "hub-local";
pub(crate) const HOST_DIRECTORY: &str = "Runtime";
#[cfg(any(target_os = "linux", test))]
pub(crate) const LINUX_APPLICATION_DIRECTORY: &str = "yolo";
#[cfg(any(target_os = "linux", test))]
pub(crate) const LINUX_HOST_DIRECTORY: &str = "runtime";
#[cfg(any(target_os = "linux", test))]
pub(crate) const LINUX_STATE_HOME_DIRECTORY: &str = ".local/state";
#[cfg(target_os = "linux")]
pub(crate) const XDG_STATE_HOME_ENVIRONMENT: &str = "XDG_STATE_HOME";
#[cfg(target_os = "linux")]
pub(crate) const HOME_ENVIRONMENT: &str = "HOME";
#[cfg(any(target_os = "linux", test))]
pub(crate) const LINUX_HOME_UNAVAILABLE: &str =
    "local Hub requires an absolute XDG_STATE_HOME or HOME directory";
pub(crate) const PROFILE_INVALID: &str = "local Hub profile has invalid or overlapping directories";
pub(crate) const SCHEMA_INVALID: &str = "unsupported local Hub profile schema";
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub(crate) const AUTOMATIC_UNSUPPORTED: &str =
    "automatic local Hub startup requires macOS or Linux; supply --hub and --socket or --endpoint";
#[cfg(target_os = "macos")]
pub(crate) const FOUNDATION_UNAVAILABLE: &str =
    "Foundation application support directory is unavailable";
