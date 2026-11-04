use super::constants::BROKER_SOCKET_STAGING_MISSING;
use super::socket_identity::{SocketIdentity, bound_socket_identity_at};
use std::ffi::CStr;
use std::io;
use std::os::fd::RawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::net::UnixListener;
use std::path::Path;

const CLEAR_THREAD_DIRECTORY: libc::c_int = -1;
const PATH_SEPARATOR_BYTE: u8 = b'/';
const BIND_THREAD_PANIC_ERROR: &str = "retained socket bind thread panicked";
const BIND_AND_RESTORE_ERROR: &str = "socket bind and thread directory restore both failed";
const SOCKET_NAME_SEPARATOR_ERROR: &str = "staged socket name contains a path separator";
const THREAD_DIRECTORY_ENTER_CONTEXT: &str = "enter retained socket directory";
const THREAD_DIRECTORY_RESTORE_CONTEXT: &str = "restore thread working directory";
#[cfg(target_os = "linux")]
const PROCESS_DESCRIPTOR_DIRECTORY: &str = "/proc/self/fd";
#[cfg(target_os = "linux")]
const DESCRIPTOR_NOT_DIRECTORY: &str = "retained socket descriptor is not a directory";

pub(super) struct RelativeBinding {
    pub(super) listener: UnixListener,
    pub(super) identity: SocketIdentity,
    pub(super) restore_error: Option<io::Error>,
}

#[cfg(test)]
impl RelativeBinding {
    pub(super) fn inject_restore_failure(&mut self, message: &'static str) {
        self.restore_error = Some(io::Error::other(message));
    }
}

pub(super) fn bind(directory: RawFd, fallback: &Path, name: &CStr) -> io::Result<RelativeBinding> {
    if name.to_bytes().contains(&PATH_SEPARATOR_BYTE) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            SOCKET_NAME_SEPARATOR_ERROR,
        ));
    }
    platform_bind(directory, fallback, name)
}

#[cfg(target_os = "macos")]
fn platform_bind(directory: RawFd, _fallback: &Path, name: &CStr) -> io::Result<RelativeBinding> {
    std::thread::scope(|scope| scope.spawn(move || bind_on_thread(directory, name)).join())
        .map_err(|_| io::Error::other(BIND_THREAD_PANIC_ERROR))?
}

#[cfg(target_os = "macos")]
fn bind_on_thread(directory: RawFd, name: &CStr) -> io::Result<RelativeBinding> {
    let thread_directory = ThreadDirectory::enter(directory)?;
    let result = UnixListener::bind(Path::new(std::ffi::OsStr::from_bytes(name.to_bytes())))
        .and_then(|listener| {
            let identity = bound_socket_identity_at(directory, name)?.ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, BROKER_SOCKET_STAGING_MISSING)
            })?;
            Ok((listener, identity))
        });
    let restore = thread_directory.restore();
    match (result, restore) {
        (Ok((listener, identity)), restore) => Ok(RelativeBinding {
            listener,
            identity,
            restore_error: restore.err(),
        }),
        (Err(error), Ok(())) => Err(error),
        (Err(error), Err(restore)) => Err(io::Error::other(format!(
            "{BIND_AND_RESTORE_ERROR}: {error}; {restore}"
        ))),
    }
}

#[cfg(target_os = "macos")]
struct ThreadDirectory {
    active: bool,
}

#[cfg(target_os = "macos")]
impl ThreadDirectory {
    fn enter(directory: RawFd) -> io::Result<Self> {
        let result = unsafe { pthread_fchdir_np(directory) };
        if result != 0 {
            return Err(io::Error::other(format!(
                "{THREAD_DIRECTORY_ENTER_CONTEXT}: {}",
                io::Error::last_os_error()
            )));
        }
        Ok(Self { active: true })
    }

    fn restore(mut self) -> io::Result<()> {
        let result = unsafe { pthread_fchdir_np(CLEAR_THREAD_DIRECTORY) };
        if result != 0 {
            return Err(io::Error::other(format!(
                "{THREAD_DIRECTORY_RESTORE_CONTEXT}: {}",
                io::Error::last_os_error()
            )));
        }
        self.active = false;
        Ok(())
    }
}

#[cfg(target_os = "macos")]
impl Drop for ThreadDirectory {
    fn drop(&mut self) {
        if self.active {
            let _ = unsafe { pthread_fchdir_np(CLEAR_THREAD_DIRECTORY) };
        }
    }
}

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn pthread_fchdir_np(directory: libc::c_int) -> libc::c_int;
}

#[cfg(target_os = "linux")]
fn platform_bind(directory: RawFd, _fallback: &Path, name: &CStr) -> io::Result<RelativeBinding> {
    let mut metadata = std::mem::MaybeUninit::<libc::stat>::uninit();
    if unsafe { libc::fstat(directory, metadata.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    if unsafe { metadata.assume_init() }.st_mode & libc::S_IFMT != libc::S_IFDIR {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            DESCRIPTOR_NOT_DIRECTORY,
        ));
    }
    let path = Path::new(PROCESS_DESCRIPTOR_DIRECTORY)
        .join(directory.to_string())
        .join(std::ffi::OsStr::from_bytes(name.to_bytes()));
    UnixListener::bind(path).and_then(|listener| {
        let identity = bound_socket_identity_at(directory, name)?.ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, BROKER_SOCKET_STAGING_MISSING)
        })?;
        Ok(RelativeBinding {
            listener,
            identity,
            restore_error: None,
        })
    })
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/socket_bind_at_tests.rs"]
mod tests;
