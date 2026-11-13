#[cfg(target_os = "macos")]
use anyhow::{Context, Result};
#[cfg(target_os = "macos")]
use sha2::{Digest, Sha256};
#[cfg(target_os = "macos")]
use std::ffi::CString;
#[cfg(target_os = "macos")]
use std::fmt::Write;
#[cfg(target_os = "macos")]
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

#[cfg(target_os = "macos")]
use super::constants::*;

#[cfg(target_os = "macos")]
unsafe extern "C" {
    static bootstrap_port: libc::mach_port_t;
    static mach_task_self_: libc::mach_port_t;
    fn bootstrap_register(
        bootstrap: libc::mach_port_t,
        name: *const libc::c_char,
        service: libc::mach_port_t,
    ) -> libc::kern_return_t;
    fn bootstrap_look_up(
        bootstrap: libc::mach_port_t,
        name: *const libc::c_char,
        service: *mut libc::mach_port_t,
    ) -> libc::kern_return_t;
    fn mach_port_allocate(
        task: libc::mach_port_t,
        right: libc::c_int,
        name: *mut libc::mach_port_t,
    ) -> libc::kern_return_t;
    fn mach_port_deallocate(
        task: libc::mach_port_t,
        name: libc::mach_port_t,
    ) -> libc::kern_return_t;
    fn mach_port_mod_refs(
        task: libc::mach_port_t,
        name: libc::mach_port_t,
        right: libc::c_int,
        delta: libc::c_int,
    ) -> libc::kern_return_t;
}

#[derive(Debug)]
pub struct CoordinationLease {
    #[cfg(target_os = "macos")]
    port: libc::mach_port_t,
}

impl CoordinationLease {
    #[cfg(target_os = "macos")]
    pub fn try_acquire(lock_path: &Path) -> Result<Option<Self>> {
        let service = service_name(lock_path)?;
        let port = allocate_receive_right()?;
        let registration = register_service(
            || unsafe { bootstrap_register(bootstrap_port, service.as_ptr(), port) },
            || service_is_registered(&service),
        );
        match registration {
            Ok(true) => Ok(Some(Self { port })),
            Ok(false) => {
                release_receive_right(port);
                Ok(None)
            }
            Err(error) => {
                release_receive_right(port);
                Err(error)
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn try_acquire(_lock_path: &Path) -> anyhow::Result<Option<Self>> {
        Ok(Some(Self {}))
    }
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RegistrationOutcome {
    Acquired,
    Busy,
    Retry,
}

#[cfg(target_os = "macos")]
fn service_name(lock_path: &Path) -> Result<CString> {
    let mut hasher = Sha256::new();
    hasher.update(lock_path.as_os_str().as_bytes());
    let mut digest = String::with_capacity(LOCK_DIGEST_HEX_BYTES);
    for byte in hasher.finalize() {
        write!(&mut digest, "{byte:02x}").expect("writing to String cannot fail");
    }
    let value = format!("{MACH_SERVICE_PREFIX}{digest}");
    anyhow::ensure!(
        value.len() < MACH_SERVICE_MAX_BYTES,
        "Mach service name is too long"
    );
    CString::new(value).context("Mach service name contains a null byte")
}

#[cfg(target_os = "macos")]
fn allocate_receive_right() -> Result<libc::mach_port_t> {
    let mut port = libc::MACH_PORT_NULL as libc::mach_port_t;
    let status = unsafe { mach_port_allocate(mach_task_self_, MACH_PORT_RIGHT_RECEIVE, &mut port) };
    anyhow::ensure!(
        status == BOOTSTRAP_SUCCESS,
        "allocate Mach receive right: {status}"
    );
    Ok(port)
}

#[cfg(target_os = "macos")]
fn register_service<R, L>(mut register: R, mut lookup: L) -> Result<bool>
where
    R: FnMut() -> libc::kern_return_t,
    L: FnMut() -> Result<bool>,
{
    match registration_outcome(register(), &mut lookup)? {
        RegistrationOutcome::Acquired => Ok(true),
        RegistrationOutcome::Busy | RegistrationOutcome::Retry => Ok(false),
    }
}

#[cfg(target_os = "macos")]
fn registration_outcome<F>(
    status: libc::kern_return_t,
    mut lookup: F,
) -> Result<RegistrationOutcome>
where
    F: FnMut() -> Result<bool>,
{
    match status {
        BOOTSTRAP_SUCCESS => Ok(RegistrationOutcome::Acquired),
        BOOTSTRAP_NAME_IN_USE | BOOTSTRAP_SERVICE_ACTIVE => Ok(RegistrationOutcome::Busy),
        BOOTSTRAP_NOT_PRIVILEGED if lookup()? => Ok(RegistrationOutcome::Busy),
        BOOTSTRAP_NOT_PRIVILEGED => Ok(RegistrationOutcome::Retry),
        _ => anyhow::bail!("register Mach coordination service: {status}"),
    }
}

#[cfg(target_os = "macos")]
fn service_is_registered(service: &CString) -> Result<bool> {
    let mut port = libc::MACH_PORT_NULL as libc::mach_port_t;
    let status = unsafe { bootstrap_look_up(bootstrap_port, service.as_ptr(), &mut port) };
    if !lookup_status_is_registered(status)? {
        return Ok(false);
    }
    let result = unsafe { mach_port_deallocate(mach_task_self_, port) };
    anyhow::ensure!(
        result == BOOTSTRAP_SUCCESS,
        "release Mach service lookup right: {result}"
    );
    Ok(true)
}

#[cfg(target_os = "macos")]
fn lookup_status_is_registered(status: libc::kern_return_t) -> Result<bool> {
    match status {
        BOOTSTRAP_SUCCESS => Ok(true),
        BOOTSTRAP_UNKNOWN_SERVICE | libc::KERN_INVALID_RIGHT => Ok(false),
        _ => anyhow::bail!("look up Mach coordination service: {status}"),
    }
}

#[cfg(target_os = "macos")]
fn release_receive_right(port: libc::mach_port_t) {
    let _ = unsafe {
        mach_port_mod_refs(
            mach_task_self_,
            port,
            MACH_PORT_RIGHT_RECEIVE,
            MACH_PORT_RELEASE_REFERENCE,
        )
    };
}

#[cfg(target_os = "macos")]
impl Drop for CoordinationLease {
    fn drop(&mut self) {
        release_receive_right(self.port);
    }
}

#[cfg(test)]
#[path = "../unit_tests/lease_tests.rs"]
mod tests;
