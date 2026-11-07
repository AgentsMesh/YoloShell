use crate::constants::NULL_DEVICE;
use crate::posix_spawn::SpawnedProcess;
use std::fs::File;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

pub(super) const SLEEP_EXECUTABLE: &str = "/bin/sleep";
pub(super) const SLEEP_ARGUMENT: &str = "5";
pub(super) const EXIT_EXECUTABLE: &str = "/usr/bin/true";
pub(super) const INVALID_SIGNAL: libc::c_int = -1;
const WAIT_ERROR: libc::pid_t = -1;
const NEW_PROCESS_GROUP: libc::pid_t = 0;

pub(super) fn bare_process(executable: &str, arguments: &[&str]) -> SpawnedProcess {
    let output = File::open(NULL_DEVICE).unwrap();
    let stderr = output.try_clone().unwrap();
    let child = Command::new(executable)
        .args(arguments)
        .process_group(NEW_PROCESS_GROUP)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    SpawnedProcess::new(child.id().try_into().unwrap(), output, stderr, None)
}

pub(super) fn assert_reaped(pid: libc::pid_t) {
    let mut status = libc::c_int::default();
    assert_eq!(
        unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) },
        WAIT_ERROR
    );
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
}
