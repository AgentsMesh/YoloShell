use anyhow::{Context, Result};
use std::ffi::{CString, OsString};
use std::os::fd::RawFd;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

#[path = "worker_spawn_environment.rs"]
mod environment;
use environment::environment_vector;

const NULL_DEVICE: &[u8] = b"/dev/null\0";
const NULL_DEVICE_MODE: libc::mode_t = 0;
const SPAWN_SUCCESS: libc::c_int = 0;
const STDIN_DESCRIPTOR: libc::c_int = libc::STDIN_FILENO;
const STDOUT_DESCRIPTOR: libc::c_int = libc::STDOUT_FILENO;
const STDERR_DESCRIPTOR: libc::c_int = libc::STDERR_FILENO;
const ARGUMENT_VECTOR_FIXED_ENTRIES: usize = 2;

pub(super) fn spawn(
    executable: &Path,
    args: &[OsString],
    overrides: &[(OsString, Option<OsString>)],
    lock_descriptors: &[RawFd],
) -> Result<libc::pid_t> {
    let executable =
        CString::new(executable.as_os_str().as_bytes()).context("encode worker executable path")?;
    let argv = argument_vector(&executable, args)?;
    let environment = environment_vector(overrides)?;
    let mut actions = FileActions::new()?;
    for descriptor in lock_descriptors {
        actions.close(*descriptor)?;
    }
    actions.open_null(STDIN_DESCRIPTOR, libc::O_RDONLY)?;
    actions.open_null(STDOUT_DESCRIPTOR, libc::O_WRONLY)?;
    actions.inherit(STDIN_DESCRIPTOR)?;
    actions.inherit(STDOUT_DESCRIPTOR)?;
    actions.inherit(STDERR_DESCRIPTOR)?;
    let attributes = SpawnAttributes::new()?;
    let mut pid = libc::pid_t::default();
    let status = unsafe {
        libc::posix_spawnp(
            &mut pid,
            executable.as_ptr(),
            &actions.value,
            &attributes.value,
            argv.pointers.as_ptr(),
            environment.pointers.as_ptr(),
        )
    };
    if status != SPAWN_SUCCESS {
        return Err(std::io::Error::from_raw_os_error(status))
            .context("spawn macOS worker with posix_spawn");
    }
    Ok(pid)
}

struct CStringVector {
    _values: Vec<CString>,
    pointers: Vec<*mut libc::c_char>,
}

fn argument_vector(executable: &CString, args: &[OsString]) -> Result<CStringVector> {
    let mut values = Vec::with_capacity(args.len() + ARGUMENT_VECTOR_FIXED_ENTRIES);
    values.push(executable.clone());
    values.extend(
        args.iter()
            .map(|arg| CString::new(arg.as_os_str().as_bytes()))
            .collect::<std::result::Result<Vec<_>, _>>()?,
    );
    let mut pointers = values
        .iter()
        .map(|value| value.as_ptr().cast_mut())
        .collect::<Vec<_>>();
    pointers.push(std::ptr::null_mut());
    Ok(CStringVector {
        _values: values,
        pointers,
    })
}

struct FileActions {
    value: libc::posix_spawn_file_actions_t,
}

impl FileActions {
    fn new() -> Result<Self> {
        let mut value = std::ptr::null_mut();
        check(
            unsafe { libc::posix_spawn_file_actions_init(&mut value) },
            "initialize worker spawn actions",
        )?;
        Ok(Self { value })
    }

    fn close(&mut self, descriptor: RawFd) -> Result<()> {
        check(
            unsafe { libc::posix_spawn_file_actions_addclose(&mut self.value, descriptor) },
            "close host lock in worker",
        )
    }

    fn open_null(&mut self, descriptor: libc::c_int, flags: libc::c_int) -> Result<()> {
        check(
            unsafe {
                libc::posix_spawn_file_actions_addopen(
                    &mut self.value,
                    descriptor,
                    NULL_DEVICE.as_ptr().cast(),
                    flags,
                    NULL_DEVICE_MODE,
                )
            },
            "redirect worker standard stream",
        )
    }

    fn inherit(&mut self, descriptor: libc::c_int) -> Result<()> {
        check(
            unsafe { posix_spawn_file_actions_addinherit_np(&mut self.value, descriptor) },
            "inherit worker standard stream",
        )
    }
}

impl Drop for FileActions {
    fn drop(&mut self) {
        unsafe { libc::posix_spawn_file_actions_destroy(&mut self.value) };
    }
}

struct SpawnAttributes {
    value: libc::posix_spawnattr_t,
}

impl SpawnAttributes {
    fn new() -> Result<Self> {
        let mut value = std::ptr::null_mut();
        check(
            unsafe { libc::posix_spawnattr_init(&mut value) },
            "initialize worker spawn attributes",
        )?;
        let flags = (libc::POSIX_SPAWN_CLOEXEC_DEFAULT as libc::c_short)
            | super::constants::MACOS_POSIX_SPAWN_SETSID_FLAG;
        if let Err(error) = check(
            unsafe { libc::posix_spawnattr_setflags(&mut value, flags) },
            "configure worker spawn session",
        ) {
            unsafe { libc::posix_spawnattr_destroy(&mut value) };
            return Err(error);
        }
        Ok(Self { value })
    }
}

impl Drop for SpawnAttributes {
    fn drop(&mut self) {
        unsafe { libc::posix_spawnattr_destroy(&mut self.value) };
    }
}

fn check(status: libc::c_int, context: &'static str) -> Result<()> {
    if status == SPAWN_SUCCESS {
        Ok(())
    } else {
        Err(std::io::Error::from_raw_os_error(status)).context(context)
    }
}

unsafe extern "C" {
    fn posix_spawn_file_actions_addinherit_np(
        actions: *mut libc::posix_spawn_file_actions_t,
        descriptor: libc::c_int,
    ) -> libc::c_int;
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/worker_spawn_macos_tests.rs"]
mod tests;
