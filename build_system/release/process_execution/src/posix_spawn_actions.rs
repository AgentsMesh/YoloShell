use crate::constants::{NULL_DEVICE, PROCESS_DEFAULT_SIGNALS};
use anyhow::{Context, Result, ensure};
use std::ffi::CString;
use std::ptr;

pub(crate) struct FileActions {
    pub value: libc::posix_spawn_file_actions_t,
}

impl FileActions {
    pub fn new() -> Result<Self> {
        let mut value = ptr::null_mut();
        check(
            unsafe { libc::posix_spawn_file_actions_init(&mut value) },
            "initialize spawn actions",
        )?;
        Ok(Self { value })
    }

    pub fn fchdir(&mut self, descriptor: libc::c_int) -> Result<()> {
        check(
            unsafe { posix_spawn_file_actions_addfchdir_np(&mut self.value, descriptor) },
            "add retained cwd",
        )
    }

    pub fn dup(&mut self, source: libc::c_int, destination: libc::c_int) -> Result<()> {
        check(
            unsafe { libc::posix_spawn_file_actions_adddup2(&mut self.value, source, destination) },
            "add descriptor mapping",
        )?;
        check(
            unsafe { posix_spawn_file_actions_addinherit_np(&mut self.value, destination) },
            "preserve mapped descriptor",
        )
    }

    pub fn open_null_stdin(&mut self) -> Result<()> {
        let path = CString::new(NULL_DEVICE)?;
        check(
            unsafe {
                libc::posix_spawn_file_actions_addopen(
                    &mut self.value,
                    libc::STDIN_FILENO,
                    path.as_ptr(),
                    libc::O_RDONLY,
                    0,
                )
            },
            "add null stdin",
        )
    }
}

impl Drop for FileActions {
    fn drop(&mut self) {
        unsafe { libc::posix_spawn_file_actions_destroy(&mut self.value) };
    }
}

pub(crate) struct SpawnAttributes {
    pub value: libc::posix_spawnattr_t,
}

impl SpawnAttributes {
    pub fn new() -> Result<Self> {
        let mut value = ptr::null_mut();
        check(
            unsafe { libc::posix_spawnattr_init(&mut value) },
            "initialize spawn attributes",
        )?;
        Ok(Self { value })
    }

    pub fn configure(&mut self) -> Result<()> {
        let flags = libc::POSIX_SPAWN_SETPGROUP
            | libc::POSIX_SPAWN_START_SUSPENDED
            | libc::POSIX_SPAWN_SETSIGMASK
            | libc::POSIX_SPAWN_SETSIGDEF
            | libc::POSIX_SPAWN_CLOEXEC_DEFAULT;
        check(
            unsafe { libc::posix_spawnattr_setflags(&mut self.value, flags as libc::c_short) },
            "set spawn flags",
        )?;
        check(
            unsafe { libc::posix_spawnattr_setpgroup(&mut self.value, 0) },
            "set spawn process group",
        )?;
        let mut signals = unsafe { std::mem::zeroed() };
        ensure!(
            unsafe { libc::sigemptyset(&mut signals) } == 0,
            "empty spawn signal set"
        );
        check(
            unsafe { libc::posix_spawnattr_setsigmask(&mut self.value, &signals) },
            "set spawn signal mask",
        )?;
        for signal in PROCESS_DEFAULT_SIGNALS {
            ensure!(
                unsafe { libc::sigaddset(&mut signals, *signal) } == 0,
                "add spawn default signal"
            );
        }
        check(
            unsafe { libc::posix_spawnattr_setsigdefault(&mut self.value, &signals) },
            "set spawn signal defaults",
        )
    }
}

impl Drop for SpawnAttributes {
    fn drop(&mut self) {
        unsafe { libc::posix_spawnattr_destroy(&mut self.value) };
    }
}

fn check(status: libc::c_int, context: &str) -> Result<()> {
    if status == 0 {
        Ok(())
    } else {
        Err(std::io::Error::from_raw_os_error(status)).context(context.to_owned())
    }
}

unsafe extern "C" {
    fn posix_spawn_file_actions_addinherit_np(
        actions: *mut libc::posix_spawn_file_actions_t,
        descriptor: libc::c_int,
    ) -> libc::c_int;
    fn posix_spawn_file_actions_addfchdir_np(
        actions: *mut libc::posix_spawn_file_actions_t,
        descriptor: libc::c_int,
    ) -> libc::c_int;
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/spawn_actions.rs"]
mod unit_tests;
