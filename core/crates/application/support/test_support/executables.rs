#[cfg(target_os = "linux")]
pub const TRUE: &str = "/bin/true";
#[cfg(not(target_os = "linux"))]
pub const TRUE: &str = "/usr/bin/true";

#[cfg(target_os = "linux")]
pub const FALSE: &str = "/bin/false";
#[cfg(not(target_os = "linux"))]
pub const FALSE: &str = "/usr/bin/false";

pub const CAT: &str = "/bin/cat";

#[cfg(target_os = "linux")]
pub const TRUE_RELATIVE: &str = "bin/true";
#[cfg(not(target_os = "linux"))]
pub const TRUE_RELATIVE: &str = "usr/bin/true";

#[cfg(test)]
#[path = "../unit_tests/test_executables_tests.rs"]
mod tests;
