use super::constants::*;
use crate::hub::federation_support::{RunningNode, deadline};
use std::{path::Path, process::Stdio};
use tokio::{io::AsyncReadExt, time::timeout_at};
use yolo_hub_identity::{AgentTarget, HubId, RuntimeTarget};

#[derive(serde::Serialize)]
struct Input {
    connection: yolo_hub_local_startup::ConnectionDocument,
    terminal: RuntimeTarget,
    agent: AgentTarget,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub entry_hub: HubId,
    pub terminal: RuntimeTarget,
    pub agent: AgentTarget,
    pub request_id: uuid::Uuid,
    pub helper_process_id: i32,
    pub helper_unique_id: u64,
    pub readonly_blocked: bool,
    pub sent: bool,
    pub helper_unchanged: bool,
    pub launch_unchanged: bool,
    pub selection_unchanged: bool,
    pub focus_unchanged: bool,
}

pub async fn run(
    node: &RunningNode,
    root: &Path,
    terminal: RuntimeTarget,
    agent: AgentTarget,
) -> Report {
    let input = root.join(INPUT_FILE);
    let report = root.join(REPORT_FILE);
    yolo_runtime::paths::atomic_write_json(
        &input,
        &Input {
            connection: yolo_hub_local_startup::ConnectionDocument::local(
                node.node.hub_id(),
                node.socket.clone(),
            ),
            terminal,
            agent,
        },
    )
    .unwrap();
    let binary = |name| crate::bazel_runfile(&std::env::var(name).unwrap()).unwrap();
    let mut child = tokio::process::Command::new(binary(PROBE_ENV))
        .arg(input)
        .arg(binary(ATTACH_ENV))
        .arg(&report)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut errors = child.stderr.take().unwrap().take(MAX_DIAGNOSTIC_BYTES);
    let mut diagnostic = Vec::new();
    let result = timeout_at(deadline(), async {
        tokio::try_join!(child.wait(), errors.read_to_end(&mut diagnostic))
    })
    .await;
    match result {
        Ok(Ok((status, _))) => assert!(
            status.success(),
            "native Agent control failed: {}",
            String::from_utf8_lossy(&diagnostic)
        ),
        result => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            panic!(
                "native Agent control did not finish: {result:?}; {}",
                String::from_utf8_lossy(&diagnostic)
            );
        }
    }
    serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap()
}
