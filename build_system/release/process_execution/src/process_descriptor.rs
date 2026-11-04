use crate::constants::FCNTL_ERROR;
use anyhow::{Context, Result};

pub(crate) fn set_close_on_exec(descriptor: libc::c_int) -> Result<()> {
    set_flags(
        descriptor,
        libc::F_GETFD,
        libc::F_SETFD,
        libc::FD_CLOEXEC,
        "protect process pipe descriptor",
    )
}

pub(crate) fn set_nonblocking(descriptor: libc::c_int) -> Result<()> {
    set_flags(
        descriptor,
        libc::F_GETFL,
        libc::F_SETFL,
        libc::O_NONBLOCK,
        "set child pipe nonblocking",
    )
}

fn set_flags(
    descriptor: libc::c_int,
    get_operation: libc::c_int,
    set_operation: libc::c_int,
    additional_flags: libc::c_int,
    description: &str,
) -> Result<()> {
    let flags = unsafe { libc::fcntl(descriptor, get_operation) };
    if flags == FCNTL_ERROR
        || unsafe { libc::fcntl(descriptor, set_operation, flags | additional_flags) }
            == FCNTL_ERROR
    {
        return Err(std::io::Error::last_os_error()).context(description.to_owned());
    }
    Ok(())
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/descriptor_flags.rs"]
mod unit_tests;
