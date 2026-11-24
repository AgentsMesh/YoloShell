use super::constants::*;
use std::{
    ffi::OsStr,
    io::Read,
    os::unix::process::CommandExt,
    process::{Command, Stdio},
};
use tokio::time::{Instant, timeout_at};
use yolo_process::ScopedWaitableChild;

pub struct Output {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub async fn run(arguments: &[&str], program_name: Option<&str>) -> Output {
    let root = tempfile::tempdir().unwrap();
    let stdout = tempfile::NamedTempFile::new_in(root.path()).unwrap();
    let stderr = tempfile::NamedTempFile::new_in(root.path()).unwrap();
    let executable = std::fs::canonicalize(std::env::var_os(BINARY_ENV).unwrap()).unwrap();
    let mut command = Command::new(executable);
    command
        .args(arguments)
        .current_dir(root.path())
        .stdin(Stdio::null())
        .stdout(stdout.reopen().unwrap())
        .stderr(stderr.reopen().unwrap());
    if let Some(name) = program_name {
        command.arg0(OsStr::new(name));
    }
    let deadline = Instant::now() + DEADLINE;
    let mut child =
        ScopedWaitableChild::spawn_before_deadline(&mut command, deadline.into_std()).unwrap();
    drop(command);
    let status = timeout_at(deadline, async {
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                return status;
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap();
    child.retire_reaped().unwrap();
    Output {
        code: status.code().unwrap(),
        stdout: read(&stdout),
        stderr: read(&stderr),
    }
}

fn read(file: &tempfile::NamedTempFile) -> String {
    let mut bytes = Vec::new();
    file.reopen()
        .unwrap()
        .take(MAX_OUTPUT + OVERFLOW_SENTINEL)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() as u64 <= MAX_OUTPUT);
    String::from_utf8(bytes).unwrap()
}
