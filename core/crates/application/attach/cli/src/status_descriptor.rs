use super::super::constants::{
    STATUS_FD_FLAGS_CONTEXT, STATUS_FD_NOT_STREAM, STATUS_FD_NOT_UNIX, STATUS_FD_OPEN_CONTEXT,
    STATUS_FD_PEER_ADDRESS_LENGTH, STATUS_FD_PEER_CONTEXT, STATUS_FD_RESERVED,
    STATUS_FD_SOCKET_TYPE_CONTEXT, STATUS_FD_SOCKET_TYPE_LENGTH, STATUS_FD_STREAM_CONTEXT,
};
use anyhow::{Context, Result, ensure};
use std::mem::{offset_of, size_of};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use yolo_runtime::local_unix::LocalUnixStream;

const FCNTL_FAILURE: libc::c_int = -1;
const SOCKET_CALL_FAILURE: libc::c_int = -1;

pub(super) fn adopt(raw: RawFd) -> Result<LocalUnixStream> {
    ensure!(raw > libc::STDERR_FILENO, STATUS_FD_RESERVED);
    let descriptor_flags = fcntl_get(raw, libc::F_GETFD).context(STATUS_FD_OPEN_CONTEXT)?;
    // SAFETY: F_GETFD proved the transferred descriptor is open and this call takes its ownership.
    let descriptor = unsafe { OwnedFd::from_raw_fd(raw) };
    validate_connected_unix_stream(&descriptor)?;
    fcntl_set(
        raw,
        libc::F_SETFD,
        descriptor_flags | libc::FD_CLOEXEC,
    )
    .context(STATUS_FD_FLAGS_CONTEXT)?;
    let status_flags = fcntl_get(raw, libc::F_GETFL).context(STATUS_FD_FLAGS_CONTEXT)?;
    fcntl_set(raw, libc::F_SETFL, status_flags | libc::O_NONBLOCK)
        .context(STATUS_FD_FLAGS_CONTEXT)?;
    let stream = std::os::unix::net::UnixStream::from(descriptor);
    LocalUnixStream::from_std(stream).context(STATUS_FD_STREAM_CONTEXT)
}

fn validate_connected_unix_stream(descriptor: &OwnedFd) -> Result<()> {
    let raw = descriptor.as_raw_fd();
    let socket_type = socket_type(raw).context(STATUS_FD_SOCKET_TYPE_CONTEXT)?;
    ensure!(socket_type == libc::SOCK_STREAM, STATUS_FD_NOT_STREAM);
    let family = peer_family(raw).context(STATUS_FD_PEER_CONTEXT)?;
    ensure!(libc::c_int::from(family) == libc::AF_UNIX, STATUS_FD_NOT_UNIX);
    Ok(())
}

fn socket_type(raw: RawFd) -> Result<libc::c_int> {
    let mut value = libc::c_int::default();
    let mut length = libc::socklen_t::try_from(size_of::<libc::c_int>()).unwrap();
    let result = unsafe {
        libc::getsockopt(
            raw,
            libc::SOL_SOCKET,
            libc::SO_TYPE,
            (&raw mut value).cast(),
            &mut length,
        )
    };
    if result == SOCKET_CALL_FAILURE {
        return Err(std::io::Error::last_os_error().into());
    }
    ensure!(
        usize::try_from(length).ok() == Some(size_of::<libc::c_int>()),
        STATUS_FD_SOCKET_TYPE_LENGTH
    );
    Ok(value)
}

fn peer_family(raw: RawFd) -> Result<libc::sa_family_t> {
    let mut address = std::mem::MaybeUninit::<libc::sockaddr_storage>::zeroed();
    let mut length = libc::socklen_t::try_from(size_of::<libc::sockaddr_storage>()).unwrap();
    let result = unsafe {
        libc::getpeername(raw, address.as_mut_ptr().cast::<libc::sockaddr>(), &mut length)
    };
    if result == SOCKET_CALL_FAILURE {
        return Err(std::io::Error::last_os_error().into());
    }
    let required = offset_of!(libc::sockaddr_storage, ss_family) + size_of::<libc::sa_family_t>();
    ensure!(
        usize::try_from(length).is_ok_and(|actual| actual >= required),
        STATUS_FD_PEER_ADDRESS_LENGTH
    );
    // SAFETY: getpeername succeeded and the returned length includes ss_family.
    Ok(unsafe { address.assume_init() }.ss_family)
}

fn fcntl_get(raw: RawFd, command: libc::c_int) -> std::io::Result<libc::c_int> {
    let result = unsafe { libc::fcntl(raw, command) };
    (result != FCNTL_FAILURE)
        .then_some(result)
        .ok_or_else(std::io::Error::last_os_error)
}

fn fcntl_set(raw: RawFd, command: libc::c_int, flags: libc::c_int) -> std::io::Result<()> {
    let result = unsafe { libc::fcntl(raw, command, flags) };
    (result != FCNTL_FAILURE)
        .then_some(())
        .ok_or_else(std::io::Error::last_os_error)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))] #[path = "../unit_tests/status_descriptor_tests.rs"]
mod tests;
