use std::time::Duration;
pub(crate) const PROBE_PROGRAM: &str = "/usr/bin/uname";
pub(crate) const PLATFORM_ARGUMENT: &str = "-s";
pub(crate) const ARCHITECTURE_ARGUMENT: &str = "-m";
pub(crate) const VERSION_ARGUMENT: &str = "--version";
pub(crate) const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);
pub(crate) const SUCCESS: u32 = 0;
pub(crate) const PLATFORM_LINUX: &str = "Linux";
pub(crate) const PLATFORM_MACOS: &str = "Darwin";
pub(crate) const ARCH_ARM: &[&str] = &["aarch64", "arm64"];
pub(crate) const ARCH_X86: &[&str] = &["x86_64", "amd64"];
pub(crate) const INVALID_CHANNEL: &str = "bundle operation used an unrelated SSH channel";
pub(crate) const COMMAND_FAILED: &str =
    "bundle command did not complete with bounded successful output";
pub(crate) const TARGET_MISMATCH: &str = "remote platform or architecture differs from bundle";
pub(crate) const VERSION_MISMATCH: &str =
    "remote executable identity or version differs from bundle";
pub(crate) const STAGE_MISMATCH: &str = "bundle stage result differs from requested deployment";
pub(crate) const REQUEST_MISMATCH: &str = "publication request differs from validated bundle stage";
pub(crate) const SELECTION_CONFLICT: &str = "remote selection changed before bundle deployment";
pub(crate) const RECOVERY_MISMATCH: &str = "read-only inspection did not confirm this publication";
pub(crate) const INVALID_PATH: &str = "bundle executable path is not valid UTF-8";
pub(crate) const UNCERTAIN: &str =
    "remote publication outcome is uncertain; do not replay the commit";
