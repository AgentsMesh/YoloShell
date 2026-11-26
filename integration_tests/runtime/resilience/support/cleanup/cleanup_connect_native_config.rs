use super::*;

pub(super) fn configure_socket(descriptor: RawFd) -> io::Result<()> {
    set_no_sigpipe(descriptor)
}

#[cfg(target_vendor = "apple")]
fn set_no_sigpipe(descriptor: RawFd) -> io::Result<()> {
    const NO_SIGPIPE_OPTION: libc::c_int = 1;
    const SYSCALL_SUCCESS: libc::c_int = 0;
    let option = NO_SIGPIPE_OPTION;
    let result = unsafe {
        libc::setsockopt(
            descriptor,
            libc::SOL_SOCKET,
            libc::SO_NOSIGPIPE,
            (&raw const option).cast(),
            libc::socklen_t::try_from(size_of_val(&option)).unwrap(),
        )
    };
    (result == SYSCALL_SUCCESS)
        .then_some(())
        .ok_or_else(io::Error::last_os_error)
}

#[cfg(not(target_vendor = "apple"))]
fn set_no_sigpipe(_descriptor: RawFd) -> io::Result<()> {
    Ok(())
}
