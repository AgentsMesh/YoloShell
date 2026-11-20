use crate::host::tests::*;
fn clear_instance_fence_close_on_exec(
    descriptors: [std::os::fd::RawFd; HOST_INSTANCE_LOCK_COUNT],
) -> std::io::Result<()> {
    for descriptor in descriptors {
        let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
        if flags == INSTANCE_FENCE_SYSCALL_FAILURE {
            return Err(std::io::Error::last_os_error());
        }
        if unsafe { libc::fcntl(descriptor, libc::F_SETFD, flags & !libc::FD_CLOEXEC) }
            == INSTANCE_FENCE_SYSCALL_FAILURE
        {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(())
}

fn instance_fence_root() -> PathBuf {
    std::env::var_os(INSTANCE_FENCE_ROOT_ENV)
        .map(PathBuf::from)
        .unwrap()
}
