use super::model::{Inspection, valid_id};
use anyhow::{Result, ensure};
use yolo_release_contracts::*;
use yolo_release_gate_system::GateSnapshot;
use yolo_release_process_execution::ProcessCommand;
use yolo_release_tool_custody::DockerClientTool;

pub(crate) enum Selector<'a> {
    Name(&'a str),
    Id(&'a str),
}
pub(crate) trait Engine {
    fn identity(&self) -> Result<String>;
    fn find(&self, selector: Selector<'_>) -> Result<Option<Inspection>>;
    fn remove(&self, id: &str) -> Result<()>;
}

pub(crate) struct SystemEngine<'a> {
    pub snapshot: &'a GateSnapshot,
    pub docker: &'a dyn DockerClientTool,
}
impl SystemEngine<'_> {
    fn output(&self, arguments: Vec<String>) -> Result<Vec<u8>> {
        self.docker.ensure_unchanged()?;
        let output = ProcessCommand::new(
            self.docker.path(),
            arguments,
            self.snapshot.source(),
            self.snapshot.home(),
            self.snapshot.temporary(),
            CONTAINER_CONTROL_FAILED,
        )
        .retain_directories(self.docker.resource_directory())
        .with_timeout(CONTAINER_CONTROL_TIMEOUT)
        .with_output_limit(CONTAINER_CONTROL_OUTPUT_BYTES)
        .execute()?;
        self.docker.ensure_unchanged()?;
        ensure!(
            output.status.success(),
            "{CONTAINER_CONTROL_FAILED}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(output.stdout)
    }
}
impl Engine for SystemEngine<'_> {
    fn identity(&self) -> Result<String> {
        let bytes = self.output(vec![
            ENGINE_INFO_COMMAND.into(),
            ENGINE_FORMAT_ARGUMENT.into(),
            ENGINE_ID_FORMAT.into(),
        ])?;
        let identity = std::str::from_utf8(&bytes)?.trim();
        ensure!(
            !identity.is_empty() && identity.bytes().all(|byte| byte.is_ascii_graphic()),
            ENGINE_CHANGED
        );
        Ok(identity.into())
    }
    fn find(&self, selector: Selector<'_>) -> Result<Option<Inspection>> {
        let filter = match selector {
            Selector::Name(name) => {
                format!("{CONTAINER_NAME_FILTER_PREFIX}{name}{CONTAINER_NAME_FILTER_SUFFIX}")
            }
            Selector::Id(id) => {
                ensure!(valid_id(id), CONTAINER_CHANGED);
                format!("{CONTAINER_ID_FILTER_PREFIX}{id}")
            }
        };
        let bytes = self.output(vec![
            CONTAINER_COMMAND.into(),
            CONTAINER_LIST_COMMAND.into(),
            CONTAINER_ALL_ARGUMENT.into(),
            CONTAINER_NO_TRUNC_ARGUMENT.into(),
            CONTAINER_QUIET_ARGUMENT.into(),
            CONTAINER_FILTER_ARGUMENT.into(),
            filter,
        ])?;
        let values = std::str::from_utf8(&bytes)?.lines().collect::<Vec<_>>();
        if values.is_empty() {
            return Ok(None);
        }
        ensure!(
            values.len() == CONTAINER_EXPECTED_MATCHES && valid_id(values[0]),
            CONTAINER_CHANGED
        );
        if let Selector::Id(expected) = selector {
            ensure!(values[0] == expected, CONTAINER_CHANGED);
        }
        let bytes = self.output(vec![
            CONTAINER_COMMAND.into(),
            CONTAINER_INSPECT_COMMAND.into(),
            values[0].into(),
        ])?;
        let mut inspected: Vec<Inspection> = serde_json::from_slice(&bytes)?;
        ensure!(
            inspected.len() == CONTAINER_EXPECTED_MATCHES && inspected[0].id == values[0],
            CONTAINER_CHANGED
        );
        Ok(inspected.remove(0).into())
    }
    fn remove(&self, id: &str) -> Result<()> {
        ensure!(valid_id(id), CONTAINER_CHANGED);
        self.output(vec![
            CONTAINER_COMMAND.into(),
            CONTAINER_REMOVE_COMMAND.into(),
            CONTAINER_FORCE_ARGUMENT.into(),
            id.into(),
        ])?;
        Ok(())
    }
}
