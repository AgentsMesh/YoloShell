use crate::constants::*;
use crate::error::WorktreeError;
use std::io::Read;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use yolo_process::DetachedChildCustody;

pub(crate) const MAX_GIT_OUTPUT_BYTES: u64 = 1_024 * 1_024;
pub(crate) const GIT_TIMEOUT: Duration = Duration::from_secs(30);

/// Run git with a fixed environment, argv only, a hard deadline and bounded
/// output. Hooks are the reason `trust_repository` is a separate precondition.
pub(crate) fn run(executable: &Path, args: &[String], cwd: &Path) -> Result<String, WorktreeError> {
    run_with_deadline(executable, args, cwd, GIT_TIMEOUT)
}

pub(crate) fn run_with_deadline(
    executable: &Path,
    args: &[String],
    cwd: &Path,
    timeout: Duration,
) -> Result<String, WorktreeError> {
    let mut command = Command::new(executable);
    command
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LC_ALL", "C")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("HOME", cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let _spawn_guard = yolo_process_spawn::waitable_child_spawn_guard()
        .map_err(|error| WorktreeError::Io(error.to_string()))?;
    let mut child = command
        .spawn()
        .map_err(|error| WorktreeError::GitFailed(error.to_string()))?;
    let stdout = child.stdout.take().map(reader);
    let stderr = child.stderr.take().map(reader);
    let mut custody = match DetachedChildCustody::capture(child) {
        Ok(custody) => custody,
        Err(error) => {
            let (source, child) = error.into_parts();
            let _ = yolo_process::terminate_detached_child(child);
            return Err(WorktreeError::Unknown(source.to_string()));
        }
    };
    let deadline = Instant::now() + timeout;
    let status = loop {
        match custody.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(GIT_POLL_INTERVAL);
            }
            Ok(None) => {
                if !terminate_group(&custody) {
                    let _ = custody.request_force_termination();
                }
                let _ = custody.wait();
                return Err(WorktreeError::Timeout);
            }
            Err(error) => return Err(WorktreeError::Io(error.to_string())),
        }
    };
    let stdout = join(stdout)?;
    let stderr = join(stderr)?;
    if !status.success() {
        let message = stderr.trim();
        return Err(WorktreeError::GitFailed(if message.is_empty() {
            format!("git exited with {status}")
        } else {
            message.to_owned()
        }));
    }
    Ok(stdout)
}

fn terminate_group(custody: &DetachedChildCustody) -> bool {
    let Some(identity) = custody.identity() else {
        return false;
    };
    if !identity.matches() {
        return false;
    }
    let Ok(pid) = libc::pid_t::try_from(custody.process_id()) else {
        return false;
    };
    if unsafe { libc::getpgid(pid) } != pid {
        return false;
    }
    unsafe { libc::kill(-pid, libc::SIGKILL) == 0 }
}

fn reader<R: Read + Send + 'static>(input: R) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut output = Vec::new();
        let _ = input
            .take(MAX_GIT_OUTPUT_BYTES - 1)
            .read_to_end(&mut output);
        output
    })
}

fn join(handle: Option<std::thread::JoinHandle<Vec<u8>>>) -> Result<String, WorktreeError> {
    let Some(handle) = handle else {
        return Ok(String::new());
    };
    let bytes = handle
        .join()
        .map_err(|_| WorktreeError::Io("git output reader panicked".into()))?;
    String::from_utf8(bytes).map_err(|_| WorktreeError::OutputTooLarge)
}
