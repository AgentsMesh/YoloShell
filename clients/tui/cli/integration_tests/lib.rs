mod argument_failures;
mod automatic_startup;
#[path = "../test_support/constants.rs"]
mod constants;
use constants::*;
use std::{
    io::Read,
    process::{Command, Stdio},
};
use tokio::time::{Instant, timeout_at};
use yolo_hub_client::ClientError;
use yolo_process::ScopedWaitableChild;

#[tokio::test]
async fn unsupported_entry_exits_before_workspace_creation_and_terminal_setup() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = directory.path().join(WORKSPACE_NAME);
    let output = tempfile::NamedTempFile::new_in(directory.path()).unwrap();
    let diagnostic = tempfile::NamedTempFile::new_in(directory.path()).unwrap();
    let executable = std::env::var_os(BINARY_ENV).unwrap();
    let mut command = Command::new(executable);
    command
        .args([HUB_ARGUMENT, HUB, ENDPOINT_ARGUMENT, SSH_ENDPOINT])
        .stdin(Stdio::null())
        .stdout(output.reopen().unwrap())
        .stderr(diagnostic.reopen().unwrap());
    let deadline = Instant::now() + PROCESS_DEADLINE;
    let mut child =
        ScopedWaitableChild::spawn_detached_before_deadline(&mut command, deadline.into_std())
            .unwrap();
    drop(command);
    let status = timeout_at(deadline, async {
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                return status;
            }
            tokio::time::sleep(PROCESS_POLL).await;
        }
    })
    .await
    .unwrap();
    child.retire_reaped().unwrap();
    assert_eq!(status.code(), Some(USAGE_FAILURE));
    assert!(capture(&output).is_empty());
    let stderr = String::from_utf8(capture(&diagnostic)).unwrap();
    assert!(
        stderr.contains(&ClientError::UnsupportedEndpoint.to_string()),
        "{stderr}"
    );
    assert!(!workspace.exists());
}

fn capture(file: &tempfile::NamedTempFile) -> Vec<u8> {
    let mut bytes = Vec::new();
    file.reopen()
        .unwrap()
        .take(MAX_CLI_OUTPUT_BYTES + OVERFLOW_PROBE_BYTES)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() as u64 <= MAX_CLI_OUTPUT_BYTES);
    bytes
}
