#[cfg(target_os = "linux")]
pub(crate) const TRUE: &str = "/bin/true";
#[cfg(not(target_os = "linux"))]
pub(crate) const TRUE: &str = "/usr/bin/true";
