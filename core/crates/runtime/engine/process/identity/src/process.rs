#[cfg(test)]
pub(crate) use crate::test_support::spawn_waitable_test_child;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use anyhow::Context;
use anyhow::Result;
include!("process_constants.rs");

#[cfg(target_os = "linux")]
#[path = "linux_process.rs"]
pub(crate) mod linux;
#[cfg(target_os = "linux")]
use linux::linux_process_stat;
#[cfg(all(test, target_os = "linux"))]
use linux::parse_linux_process_stat;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessIdentityObservation {
    Matching,
    Gone,
    Unknown,
}

pub type ProcessIdentityRelation = ProcessIdentityObservation;

pub fn process_identity_relation(pid: u32, expected_start_time: u64) -> ProcessIdentityRelation {
    observe_process_identity(pid, expected_start_time)
}

pub fn observe_process_identity(pid: u32, expected_start_time: u64) -> ProcessIdentityObservation {
    #[cfg(any(test, feature = "identity-test-support"))]
    if let Some(observation) =
        crate::identity_relation_test_support::observation(pid, expected_start_time)
    {
        return observation;
    }
    #[cfg(target_os = "macos")]
    return match super::macos_kernel::metadata(pid) {
        Ok(Some(metadata)) if metadata.start_time == expected_start_time => {
            ProcessIdentityObservation::Matching
        }
        Ok(Some(_)) | Ok(None) => ProcessIdentityObservation::Gone,
        Err(_) => ProcessIdentityObservation::Unknown,
    };
    #[cfg(target_os = "linux")]
    return match linux_process_stat(pid) {
        Ok(stat) if stat.start_time == expected_start_time && stat.is_active() => {
            ProcessIdentityObservation::Matching
        }
        Ok(_) => ProcessIdentityObservation::Gone,
        Err(error) if io_kind(&error) == Some(std::io::ErrorKind::NotFound) => {
            ProcessIdentityObservation::Gone
        }
        Err(_) => ProcessIdentityObservation::Unknown,
    };
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    match try_process_identity_matches(pid, expected_start_time) {
        Ok(true) => ProcessIdentityObservation::Matching,
        Ok(false) => ProcessIdentityObservation::Gone,
        Err(_) => ProcessIdentityObservation::Unknown,
    }
}
pub fn process_start_time(pid: u32) -> Result<u64> {
    #[cfg(target_os = "linux")]
    {
        return linux_process_stat(pid).map(|stat| stat.start_time);
    }
    #[cfg(target_os = "macos")]
    {
        super::macos_kernel::metadata(pid)?
            .with_context(|| format!("{PROCESS_NOT_RUNNING_CONTEXT}: {pid}"))
            .map(|metadata| metadata.start_time)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = pid;
        Ok(0)
    }
}

pub fn process_identity_matches(pid: u32, expected_start_time: u64) -> bool {
    try_process_identity_matches(pid, expected_start_time).unwrap_or(false)
}

pub fn try_process_identity_matches(pid: u32, expected_start_time: u64) -> Result<bool> {
    #[cfg(target_os = "linux")]
    {
        let stat = linux_process_stat(pid)?;
        return Ok(stat.start_time == expected_start_time && stat.is_active());
    }
    #[cfg(not(target_os = "linux"))]
    {
        process_start_time(pid).map(|actual| actual == expected_start_time)
    }
}

#[cfg(target_os = "linux")]
fn io_kind(error: &anyhow::Error) -> Option<std::io::ErrorKind> {
    error.chain().find_map(|source| {
        source
            .downcast_ref::<std::io::Error>()
            .map(std::io::Error::kind)
    })
}

#[cfg(target_os = "linux")]
pub fn process_identity_is_inactive(pid: u32, expected_start_time: u64) -> Result<bool> {
    let stat = linux_process_stat(pid)?;
    Ok(stat.start_time == expected_start_time && !stat.is_active())
}

pub fn is_process_alive(pid: u32) -> bool {
    #[cfg(target_os = "linux")]
    if let Ok(stat) = linux_process_stat(pid) {
        return stat.is_active();
    }
    #[cfg(target_os = "macos")]
    if let Ok(metadata) = super::macos_kernel::metadata(pid) {
        return metadata.is_some();
    }
    #[cfg(unix)]
    unsafe {
        if pid == 0 || pid > i32::MAX as u32 {
            return false;
        }
        libc::kill(pid as i32, 0) == 0
            || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

#[cfg(all(test, target_os = "linux"))]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../integration_tests/identity_linux_tests.rs"]
mod linux_tests;

#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../integration_tests/identity_macos_tests.rs"]
mod macos_tests;
