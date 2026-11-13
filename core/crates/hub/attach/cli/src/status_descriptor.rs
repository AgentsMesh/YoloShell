use crate::{CliError, constants::SYSCALL_FAILURE};
use std::{
    mem::{offset_of, size_of},
    os::fd::{FromRawFd, OwnedFd},
};
use yolo_local_unix::LocalUnixStream;

pub(crate) fn adopt(raw: libc::c_int) -> Result<LocalUnixStream, CliError> {
    if raw <= libc::STDERR_FILENO {
        return Err(CliError::StatusDescriptor);
    }
    let flags = unsafe { libc::fcntl(raw, libc::F_GETFD) };
    if flags == SYSCALL_FAILURE {
        return Err(CliError::StatusDescriptor);
    }
    // The caller transfers this inherited descriptor exactly once.
    let owned = unsafe { OwnedFd::from_raw_fd(raw) };
    let mut kind = libc::c_int::default();
    let mut length = size_of::<libc::c_int>() as libc::socklen_t;
    if unsafe {
        libc::getsockopt(
            raw,
            libc::SOL_SOCKET,
            libc::SO_TYPE,
            (&raw mut kind).cast(),
            &mut length,
        )
    } == SYSCALL_FAILURE
        || kind != libc::SOCK_STREAM
        || length as usize != size_of::<libc::c_int>()
    {
        return Err(CliError::StatusDescriptor);
    }
    let mut address = std::mem::MaybeUninit::<libc::sockaddr_storage>::zeroed();
    let mut length = size_of::<libc::sockaddr_storage>() as libc::socklen_t;
    if unsafe { libc::getpeername(raw, address.as_mut_ptr().cast(), &mut length) }
        == SYSCALL_FAILURE
        || (length as usize)
            < offset_of!(libc::sockaddr_storage, ss_family) + size_of::<libc::sa_family_t>()
    {
        return Err(CliError::StatusDescriptor);
    }
    let address = unsafe { address.assume_init() };
    if i32::from(address.ss_family) != libc::AF_UNIX {
        return Err(CliError::StatusDescriptor);
    }
    if unsafe { libc::fcntl(raw, libc::F_SETFD, flags | libc::FD_CLOEXEC) } == SYSCALL_FAILURE {
        return Err(CliError::StatusDescriptor);
    }
    let stream = std::os::unix::net::UnixStream::from(owned);
    stream.set_nonblocking(true)?;
    LocalUnixStream::from_std(stream).map_err(Into::into)
}
