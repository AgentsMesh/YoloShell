mod constants;
pub use constants::*;
use std::{
    ffi::OsStr,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};
use tempfile::{NamedTempFile, TempDir};
use tokio::time::{Instant, timeout_at};
use yolo_process::ScopedWaitableChild;

pub struct CliFixture {
    root: TempDir,
    executable: PathBuf,
    captures: PathBuf,
}

impl CliFixture {
    pub fn new() -> Self {
        let logical = PathBuf::from(std::env::var_os(TEST_WORKSPACE_ENV).unwrap())
            .join(std::env::var_os(BINARY_ENV).unwrap());
        let executable = runfiles::Runfiles::create()
            .unwrap()
            .rlocation(&logical)
            .unwrap()
            .canonicalize()
            .unwrap();
        let root = tempfile::tempdir().unwrap();
        let captures = root.path().join(CAPTURE_DIRECTORY);
        std::fs::create_dir(&captures).unwrap();
        Self {
            root,
            executable,
            captures,
        }
    }

    pub fn path(&self, name: impl AsRef<Path>) -> PathBuf {
        self.root.path().join(name)
    }

    pub fn command(&self) -> Command {
        let mut command = Command::new(&self.executable);
        // Keep LLVM_PROFILE_* and process-custody environment inherited. Only
        // the working directory changes; commands receive private fixture paths.
        command.current_dir(self.root.path()).stdin(Stdio::null());
        command
    }

    pub async fn run(&self, command: &mut Command) -> Output {
        let stdout = NamedTempFile::new_in(&self.captures).unwrap();
        let stderr = NamedTempFile::new_in(&self.captures).unwrap();
        command
            .stdout(stdout.reopen().unwrap())
            .stderr(stderr.reopen().unwrap());
        let end = Instant::now() + DEADLINE;
        let mut child =
            ScopedWaitableChild::spawn_detached_before_deadline(command, end.into_std()).unwrap();
        let exit = timeout_at(end, async {
            loop {
                if let Some(status) = child.try_wait().unwrap() {
                    return status;
                }
                tokio::time::sleep(EXIT_POLL).await;
            }
        })
        .await;
        let status = match exit {
            Ok(status) => {
                child.retire_reaped().unwrap();
                status
            }
            Err(_) => {
                child
                    .terminate_and_wait_reaped((Instant::now() + DEADLINE).into_std())
                    .await
                    .unwrap();
                panic!("owned CLI child exceeded its bounded deadline");
            }
        };
        Output {
            status,
            stdout: capture(&stdout),
            stderr: capture(&stderr),
        }
    }

    pub async fn args(&self, args: &[impl AsRef<OsStr>]) -> Output {
        self.run(self.command().args(args)).await
    }
}

pub fn flag(name: &str) -> String {
    format!("{LONG_OPTION_PREFIX}{name}")
}

pub fn assert_status(output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

pub fn successful_text(output: &Output) -> &str {
    assert_status(output, SUCCESS);
    assert!(output.stderr.is_empty());
    std::str::from_utf8(&output.stdout).unwrap()
}

fn capture(file: &NamedTempFile) -> Vec<u8> {
    let mut bytes = Vec::new();
    file.reopen()
        .unwrap()
        .take(MAX_OUTPUT_BYTES + OVERFLOW_SENTINEL)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() as u64 <= MAX_OUTPUT_BYTES);
    bytes
}
