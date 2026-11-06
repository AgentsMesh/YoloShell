use crate::developer_release_constants::*;
use anyhow::{Result, bail};
use std::process::Output;
use std::sync::Arc;
use yolo_release_filesystem::{DirectoryHandle, TemporaryDirectory};
use yolo_release_process_execution::ProcessCommand;

pub fn run(
    workspace: &DirectoryHandle,
    executable: &str,
    arguments: Vec<String>,
    description: &str,
) -> Result<Output> {
    run_with_optional_file(workspace, executable, arguments, description, None)
}

pub fn run_with_file(
    workspace: &DirectoryHandle,
    executable: &str,
    arguments: Vec<String>,
    description: &str,
    file: Arc<std::fs::File>,
) -> Result<Output> {
    run_with_optional_file(workspace, executable, arguments, description, Some(file))
}

fn run_with_optional_file(
    workspace: &DirectoryHandle,
    executable: &str,
    arguments: Vec<String>,
    description: &str,
    inherited: Option<Arc<std::fs::File>>,
) -> Result<Output> {
    let scratch = TemporaryDirectory::create(DEVELOPER_RELEASE_COMMAND_PREFIX)?;
    let isolated = scratch.handle();
    let mut command = ProcessCommand::new(
        executable,
        arguments,
        workspace,
        isolated,
        isolated,
        description,
    )
    .with_timeout(DEVELOPER_RELEASE_COMMAND_TIMEOUT);
    if let Some(file) = inherited {
        command = command.inherit_as(file, PINNED_INPUT_DESCRIPTOR);
    }
    let output = command.execute()?;
    if !output.status.success() {
        bail!(
            "{DEVELOPER_RELEASE_PROCESS_ERROR}: {description}: {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(output)
}
