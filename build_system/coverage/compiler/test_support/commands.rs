use crate::constants::*;
use std::{
    path::PathBuf,
    process::{Command, Stdio},
};
use tokio::io::{AsyncRead, AsyncReadExt};
use yolo_process::ScopedWaitableChild;

pub fn binary(variable: &str) -> PathBuf {
    resolve(
        PathBuf::from(std::env::var_os(WORKSPACE_ENV).unwrap())
            .join(std::env::var_os(variable).unwrap()),
    )
}

pub fn tool(field: &str) -> PathBuf {
    let settings: serde_json::Value =
        serde_json::from_slice(&std::fs::read(binary(TOOLS_ENV)).unwrap()).unwrap();
    resolve(PathBuf::from(settings[field].as_str().unwrap()))
}

fn resolve(path: PathBuf) -> PathBuf {
    runfiles::Runfiles::create()
        .unwrap()
        .rlocation(&path)
        .unwrap_or_else(|| panic!("missing runfile {}", path.display()))
        .canonicalize()
        .unwrap()
}

async fn read(reader: impl AsyncRead + Unpin) -> Vec<u8> {
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
    let deadline = tokio::time::Instant::now() + TIMEOUT;
    let mut child =
        ScopedWaitableChild::spawn_before_deadline(&mut command, deadline.into_std()).unwrap();
    drop(command);
    let stdout = tokio::process::ChildStdout::from_std(child.take_stdout().unwrap()).unwrap();
    let stderr = tokio::process::ChildStderr::from_std(child.take_stderr().unwrap()).unwrap();
    let (status, out, err) = tokio::time::timeout_at(deadline, async {
        tokio::join!(
            child.wait_for_clean_exit(deadline.into_std()),
            read(stdout),
            read(stderr)
        )
    })
    .await
    .unwrap();
    assert!(
        status.is_ok(),
        "command failed: {status:?}: {}",
        String::from_utf8_lossy(&err)
    );
    out
}
