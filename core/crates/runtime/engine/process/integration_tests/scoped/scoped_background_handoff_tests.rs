use super::*;
use std::{io::Write, process::Stdio};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};

const HANDOFF_SCRIPT: &str = "printf 'ready\\n'; read input; printf 'complete\\n'";
const READY: &str = "ready\n";
const COMPLETE: &str = "complete\n";
const FINISH: &[u8] = b"finish\n";

#[tokio::test]
async fn background_handoff_preserves_process_until_natural_exit_and_reaps_it() {
    let mut command = Command::new(SHELL_EXECUTABLE);
    command
        .args([SHELL_COMMAND_ARGUMENT, HANDOFF_SCRIPT])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = spawn(command);
    let process_id = child.process_id();
    let mut input = child.take_stdin().unwrap();
    let output = tokio::process::ChildStdout::from_std(child.take_stdout().unwrap()).unwrap();
    let mut output = BufReader::new(output);
    let mut line = String::new();
    tokio::time::timeout(OPERATION_TIMEOUT, output.read_line(&mut line))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(line, READY);
    child.preserve_in_background().unwrap();
    input.write_all(FINISH).unwrap();
    drop(input);
    let mut completion = String::new();
    tokio::time::timeout(OPERATION_TIMEOUT, output.read_to_string(&mut completion))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(completion, COMPLETE);
    wait_until_reaped(process_id).await;
}

#[tokio::test]
async fn already_reaped_child_cannot_be_reported_as_a_running_background_service() {
    let mut child = spawn_shell(CLEAN_EXIT_SCRIPT);
    let process_id = child.process_id();
    tokio::time::timeout(OPERATION_TIMEOUT, async {
        while child.try_wait().unwrap().is_none() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(child.preserve_in_background().is_err());
    assert_reaped(process_id);
}
