use anyhow::{Context, Result, ensure};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

const PIPE_CREATE_CONTEXT: &str = "create process capability pipe";
const DESCRIPTOR_FLAGS_CONTEXT: &str = "configure process capability descriptor";
const DESCRIPTOR_DIRECTION_DETAIL: &str = "process capability descriptor direction mismatch";
const FCNTL_ERROR: libc::c_int = -1;
const PIPE_ENDPOINT_COUNT: usize = 2;
const PIPE_READER_INDEX: usize = 0;
const PIPE_WRITER_INDEX: usize = 1;

// Apple Bash closes descriptors 3 through 19 when starting an interactive login shell.
pub(in crate::process) const MINIMUM_INHERITED_WRITER_DESCRIPTOR: libc::c_int = 20;

pub(super) fn preserve_shell_writer(descriptor: OwnedFd) -> Result<OwnedFd> {
    if descriptor.as_raw_fd() >= MINIMUM_INHERITED_WRITER_DESCRIPTOR {
        return Ok(descriptor);
    }
    let duplicated = unsafe {
        libc::fcntl(
            descriptor.as_raw_fd(),
            libc::F_DUPFD_CLOEXEC,
            MINIMUM_INHERITED_WRITER_DESCRIPTOR,
        )
    };
    ensure!(duplicated != FCNTL_ERROR, std::io::Error::last_os_error());
    Ok(unsafe { OwnedFd::from_raw_fd(duplicated) })
}

#[derive(Clone, Copy)]
pub(super) enum EndpointDirection {
    Reader,
    Writer,
}

#[derive(Clone, Copy)]
pub(super) struct PipeEndpointIdentity {
    pub(super) handle: u64,
    pub(super) peer_handle: u64,
}

pub(super) fn create_private_pipe() -> Result<[OwnedFd; PIPE_ENDPOINT_COUNT]> {
    let _spawn_guard = super::waitable_child_spawn_guard()?;
    let mut descriptors = [libc::c_int::default(); PIPE_ENDPOINT_COUNT];
    ensure!(
        unsafe { libc::pipe(descriptors.as_mut_ptr()) } == libc::c_int::default(),
        "{PIPE_CREATE_CONTEXT}: {}",
        std::io::Error::last_os_error()
    );
    let reader = unsafe { OwnedFd::from_raw_fd(descriptors[PIPE_READER_INDEX]) };
    let writer = unsafe { OwnedFd::from_raw_fd(descriptors[PIPE_WRITER_INDEX]) };
    set_close_on_exec(reader.as_raw_fd(), true).context(DESCRIPTOR_FLAGS_CONTEXT)?;
    set_close_on_exec(writer.as_raw_fd(), true).context(DESCRIPTOR_FLAGS_CONTEXT)?;
    Ok([reader, writer])
}

pub(super) fn configure_endpoint(
    descriptor: &OwnedFd,
    direction: EndpointDirection,
    close_on_exec: bool,
) -> Result<PipeEndpointIdentity> {
    let raw = descriptor.as_raw_fd();
    set_close_on_exec(raw, close_on_exec).context(DESCRIPTOR_FLAGS_CONTEXT)?;
    let flags = unsafe { libc::fcntl(raw, libc::F_GETFL) };
    ensure!(flags != FCNTL_ERROR, DESCRIPTOR_FLAGS_CONTEXT);
    let expected = match direction {
        EndpointDirection::Reader => libc::O_RDONLY,
        EndpointDirection::Writer => libc::O_WRONLY,
    };
    ensure!(
        flags & libc::O_ACCMODE == expected,
        DESCRIPTOR_DIRECTION_DETAIL
    );
    ensure!(
        unsafe { libc::fcntl(raw, libc::F_SETFL, flags | libc::O_NONBLOCK) } != FCNTL_ERROR,
        DESCRIPTOR_FLAGS_CONTEXT
    );
    let endpoint = super::pty_session_proof::macos_descriptor_capability::inherited_pipe_endpoint(
        std::process::id(),
        raw,
    )?;
    Ok(PipeEndpointIdentity {
        handle: endpoint.handle,
        peer_handle: endpoint.peer_handle,
    })
}

pub(super) fn set_close_on_exec(descriptor: libc::c_int, enabled: bool) -> std::io::Result<()> {
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
    if flags == FCNTL_ERROR {
        return Err(std::io::Error::last_os_error());
    }
    let next = if enabled {
        flags | libc::FD_CLOEXEC
    } else {
        flags & !libc::FD_CLOEXEC
    };
    let status = unsafe { libc::fcntl(descriptor, libc::F_SETFD, next) };
    (status != FCNTL_ERROR)
        .then_some(())
        .ok_or_else(std::io::Error::last_os_error)
}
