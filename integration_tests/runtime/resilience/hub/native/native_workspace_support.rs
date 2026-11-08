#[path = "native_workspace_support/constants.rs"]
mod constants;
use super::support::RunningHub;
use crate::Harness;
pub use constants::*;
use std::{path::Path, process::Stdio};
use tokio::io::AsyncReadExt;
use tokio::time::{Instant, timeout_at};

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Ready {
    entry_hub: yolo_hub_identity::HubId,
    terminal_id: yolo_hub_identity::TerminalId,
}

pub async fn run(hub: &RunningHub, harness: &Harness, root: &Path) {
    let configuration = root.join(CONFIGURATION_FILE);
    let workspace = root.join(WORKSPACE_FILE);
    let report = root.join(REPORT_FILE);
    let ready = report.with_extension(READY_EXTENSION);
    let acknowledgement = report.with_extension(ACKNOWLEDGEMENT_EXTENSION);
    yolo_runtime::paths::atomic_write_json(
        &configuration,
        &serde_json::json!({
            SCHEMA_FIELD: CONFIG_SCHEMA, ENTRY_FIELD: hub.hub, SOCKET_FIELD: hub.socket,
            LOCAL_FIELD: hub.hub, WORKSPACE_FIELD: workspace,
            DIRECTORIES_FIELD: [{ HUB_FIELD: hub.hub, PATH_FIELD: harness.home.path() }],
        }),
    )
    .unwrap();
    let binary = |name| crate::bazel_runfile(&std::env::var(name).unwrap()).unwrap();
    let mut child = tokio::process::Command::new(binary(PROBE_ENV))
        .arg(configuration)
        .arg(binary(ATTACH_ENV))
        .arg(report)
        .arg(binary(HUB_ENV))
        .arg(crate::runtime_binary())
        .arg(root.join(super::support::STATE_DIRECTORY))
        .env(INACTIVE_ENV, ENABLED)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut stderr = child.stderr.take().unwrap().take(MAX_DIAGNOSTIC_BYTES);
    let deadline = Instant::now() + PROBE_TIMEOUT;
    let observation = async {
        let ready: Ready = loop {
            match std::fs::read(&ready) {
                Ok(bytes) => break serde_json::from_slice(&bytes).unwrap(),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    tokio::time::sleep(POLL_INTERVAL).await
                }
                Err(error) => panic!("native probe readiness read failed: {error}"),
            }
        };
        assert_eq!(ready.entry_hub, hub.hub);
        let view = harness
            .client()
            .get_session(ready.terminal_id.as_uuid())
            .await
            .unwrap();
        harness.track_view(&view);
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&acknowledgement)
            .unwrap();
        std::future::pending::<()>().await;
    };
    tokio::pin!(observation);
    let mut diagnostic = Vec::new();
    let mut chunk = [0; DIAGNOSTIC_CHUNK_BYTES];
    let mut stderr_open = true;
    let result = timeout_at(deadline, async {
        loop {
            tokio::select! {
                result = child.wait() => break result,
                _ = &mut observation => unreachable!(),
                read = stderr.read(&mut chunk), if stderr_open => {
                    let count = read?;
                    stderr_open = count != 0;
                    diagnostic.extend_from_slice(&chunk[..count]);
                }
            }
        }
    })
    .await;
    match result {
        Ok(status) => {
            let _ =
                tokio::time::timeout(DIAGNOSTIC_DRAIN, stderr.read_to_end(&mut diagnostic)).await;
            assert!(
                status.unwrap().success(),
                "native public-Hub workflow failed: {}",
                String::from_utf8_lossy(&diagnostic)
            );
        }
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            panic!("native public-Hub workflow exceeded its deadline");
        }
    }
}
