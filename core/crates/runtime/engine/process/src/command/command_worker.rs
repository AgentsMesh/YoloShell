use super::command::{DetachedStandardError, spawn_detached_with_stdio};
use anyhow::{Context, Result};
#[cfg(all(target_os = "macos", not(test)))]
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const INVALID_MACOS_PROCESS_ID: &str = "macOS worker returned an invalid process id";
const PROCESS_ID_CONVERSION_CONTEXT: &str = "convert macOS worker process id";
const WORKER_WAIT_CONTEXT: &str = "wait for worker process";
const REAPER_RECOVERY_CONTEXT: &str = "recover worker after reaper spawn failure";
const POST_SPAWN_REAPER_THREAD_NAME: &str = "yolo-worker-post-spawn-reap";
const POST_SPAWN_REAP_FAILED: &str = "post-spawn worker reaping failed";

pub enum WorkerProcess {
    #[cfg(all(target_os = "macos", not(test)))]
    MacosPid(libc::pid_t),
    Child(Child),
}

impl WorkerProcess {
    pub fn id(&self) -> Result<u32> {
        match self {
            #[cfg(all(target_os = "macos", not(test)))]
            Self::MacosPid(pid) => {
                anyhow::ensure!(*pid > 0, INVALID_MACOS_PROCESS_ID);
                u32::try_from(*pid).context(PROCESS_ID_CONVERSION_CONTEXT)
            }
            Self::Child(child) => Ok(child.id()),
        }
    }

    pub fn wait(&mut self) -> Result<std::process::ExitStatus> {
        match self {
            #[cfg(all(target_os = "macos", not(test)))]
            Self::MacosPid(pid) => wait_macos_pid(*pid),
            Self::Child(child) => child.wait().context(WORKER_WAIT_CONTEXT),
        }
    }

    pub fn reap_in_background(self, thread_name: String) -> Result<()> {
        let holder = Arc::new(Mutex::new(Some(self)));
        let worker = Arc::clone(&holder);
        let spawn_error = match std::thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                if let Some(mut process) = Self::take_reaper_process(&worker) {
                    let _ = process.wait();
                }
            }) {
            Ok(_) => return Ok(()),
            Err(error) => error,
        };
        let mut process = Self::take_reaper_process(&holder).context(REAPER_RECOVERY_CONTEXT)?;
        process
            .wait()
            .map(|_| ())
            .with_context(|| format!("synchronously reap worker after {spawn_error}"))
    }

    fn take_reaper_process(holder: &Mutex<Option<Self>>) -> Option<Self> {
        holder
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
    }
}

pub(crate) fn spawn_detached_command_before_deadline(
    command: Command,
    standard_error: DetachedStandardError,
    deadline: Instant,
) -> Result<Child> {
    super::command_private_input::require_budget(deadline)?;
    spawn_detached_with_stdio(
        command,
        standard_error,
        Stdio::null(),
        Stdio::null(),
        Some(deadline),
    )
}

#[cfg(all(target_os = "macos", not(test)))]
pub fn spawn_worker_before_deadline(
    executable: &Path,
    args: &[std::ffi::OsString],
    environment: &[(std::ffi::OsString, Option<std::ffi::OsString>)],
    lock_descriptors: &[std::os::fd::RawFd],
    deadline: Instant,
) -> Result<WorkerProcess> {
    super::command_private_input::require_budget(deadline)?;
    let _guard = super::waitable_child_spawn_guard_before(deadline)?;
    let pid = super::worker_spawn_macos::spawn(executable, args, environment, lock_descriptors)?;
    let process = WorkerProcess::MacosPid(pid);
    if let Err(deadline_error) = super::command_private_input::require_budget(deadline) {
        return match process.reap_in_background(POST_SPAWN_REAPER_THREAD_NAME.to_owned()) {
            Ok(()) => Err(deadline_error),
            Err(reap_error) => {
                Err(deadline_error.context(format!("{POST_SPAWN_REAP_FAILED}: {reap_error:#}")))
            }
        };
    }
    Ok(process)
}

#[cfg(any(not(target_os = "macos"), test))]
pub fn spawn_worker_before_deadline(
    command: Command,
    _lock_descriptors: &[std::os::fd::RawFd],
    standard_error: DetachedStandardError,
    deadline: Instant,
) -> Result<WorkerProcess> {
    super::command_private_input::require_budget(deadline)?;
    let child = spawn_detached_command_before_deadline(command, standard_error, deadline)?;
    Ok(WorkerProcess::Child(child))
}

#[cfg(all(target_os = "macos", not(test)))]
fn wait_macos_pid(pid: libc::pid_t) -> Result<std::process::ExitStatus> {
    let mut status = libc::c_int::default();
    loop {
        let result = unsafe { libc::waitpid(pid, &mut status, libc::c_int::default()) };
        if result == pid {
            use std::os::unix::process::ExitStatusExt;
            return Ok(std::process::ExitStatus::from_raw(status));
        }
        let error = std::io::Error::last_os_error();
        if result == -1 && error.kind() == std::io::ErrorKind::Interrupted {
            continue;
        }
        return Err(error).context(WORKER_WAIT_CONTEXT);
    }
}
