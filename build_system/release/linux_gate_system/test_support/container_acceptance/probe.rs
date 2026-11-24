use super::constants::*;
use anyhow::{Result, ensure};
use serde::Deserialize;
use yolo_release_contracts::*;
use yolo_release_gate_system::GateSnapshot;
use yolo_release_process_execution::ProcessCommand;
use yolo_release_tool_custody::DockerClientTool;

pub fn image_is_cached(
    snapshot: &GateSnapshot,
    docker: &dyn DockerClientTool,
    image: &str,
) -> Result<()> {
    output(
        snapshot,
        docker,
        vec![
            IMAGE_COMMAND.into(),
            CONTAINER_INSPECT_COMMAND.into(),
            image.into(),
        ],
    )?;
    Ok(())
}

pub fn is_running(snapshot: &GateSnapshot, docker: &dyn DockerClientTool, id: &str) -> Result<()> {
    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct State {
        running: bool,
        status: String,
    }
    let bytes = output(
        snapshot,
        docker,
        vec![
            CONTAINER_COMMAND.into(),
            CONTAINER_INSPECT_COMMAND.into(),
            ENGINE_FORMAT_ARGUMENT.into(),
            STATE_FORMAT.into(),
            id.into(),
        ],
    )?;
    let state: State = serde_json::from_slice(&bytes)?;
    ensure!(
        state.running && state.status == RUNNING_STATUS,
        "container did not survive CLI settlement"
    );
    Ok(())
}

fn output(
    snapshot: &GateSnapshot,
    docker: &dyn DockerClientTool,
    arguments: Vec<String>,
) -> Result<Vec<u8>> {
    docker.ensure_unchanged()?;
    let output = ProcessCommand::new(
        docker.path(),
        arguments,
        snapshot.source(),
        snapshot.home(),
        snapshot.temporary(),
        "container owner acceptance read-only engine probe",
    )
    .retain_directories(docker.resource_directory())
    .with_timeout(CONTAINER_CONTROL_TIMEOUT)
    .with_output_limit(CONTAINER_CONTROL_OUTPUT_BYTES)
    .execute()?;
    docker.ensure_unchanged()?;
    ensure!(
        output.status.success(),
        "engine probe failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(output.stdout)
}
