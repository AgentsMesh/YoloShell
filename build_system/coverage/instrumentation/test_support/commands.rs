use crate::constants::*;
use std::process::{Command, Stdio};
use tokio::io::{AsyncRead, AsyncReadExt};
use yolo_process::ScopedWaitableChild;

pub async fn read(reader: impl AsyncRead + Unpin) -> Vec<u8> {
    let mut bytes = Vec::new();
    reader
        .take(MAX_OUTPUT)
        .read_to_end(&mut bytes)
        .await
        .unwrap();
    bytes
}

pub async fn output(mut command: Command) -> Vec<u8> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let limit = tokio::time::Instant::now() + TIMEOUT;
    let mut child =
        ScopedWaitableChild::spawn_before_deadline(&mut command, limit.into_std()).unwrap();
    drop(command);
    let stdout = tokio::process::ChildStdout::from_std(child.take_stdout().unwrap()).unwrap();
    let stderr = tokio::process::ChildStderr::from_std(child.take_stderr().unwrap()).unwrap();
    let (status, bytes, errors) = tokio::time::timeout_at(limit, async {
        tokio::join!(
            child.wait_for_clean_exit(limit.into_std()),
            read(stdout),
            read(stderr)
        )
    })
    .await
    .unwrap();
    assert!(
        status.is_ok(),
        "{COMMAND_FAILURE}: {status:?}: {}",
        String::from_utf8_lossy(&errors)
    );
    bytes
}
