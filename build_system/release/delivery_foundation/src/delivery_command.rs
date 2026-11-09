use anyhow::{Result, bail};
use yolo_release_contracts::ARCHIVE_NAME;
use yolo_release_contracts::*;
use yolo_release_filesystem::DirectoryHandle;
use yolo_release_process_execution::ProcessCommand;

pub trait DeliveryCommandRunner {
    fn extract(&self, stage: &DirectoryHandle) -> Result<()>;
    fn verify_signature(&self, extraction: &DirectoryHandle) -> Result<()>;
}

pub struct SystemDeliveryCommandRunner;

impl DeliveryCommandRunner for SystemDeliveryCommandRunner {
    fn extract(&self, stage: &DirectoryHandle) -> Result<()> {
        execute(
            stage,
            DITTO_EXECUTABLE,
            extraction_arguments(),
            DELIVERY_EXTRACT_DESCRIPTION,
        )
    }

    fn verify_signature(&self, extraction: &DirectoryHandle) -> Result<()> {
        execute(
            extraction,
            CODESIGN_EXECUTABLE,
            signature_arguments(),
            DELIVERY_SIGNATURE_DESCRIPTION,
        )
    }
}

pub fn extraction_arguments() -> Vec<String> {
    DITTO_ARGUMENTS
        .iter()
        .copied()
        .chain([ARCHIVE_NAME, EXTRACTION_DIRECTORY_NAME])
        .map(str::to_owned)
        .collect()
}

pub fn signature_arguments() -> Vec<String> {
    CODESIGN_ARGUMENTS
        .iter()
        .copied()
        .chain([APP_NAME])
        .map(str::to_owned)
        .collect()
}

fn execute(
    workspace: &DirectoryHandle,
    executable: &str,
    arguments: Vec<String>,
    description: &str,
) -> Result<()> {
    let output = ProcessCommand::new(
        executable,
        arguments,
        workspace,
        workspace,
        workspace,
        description,
    )
    .with_timeout(DELIVERY_COMMAND_TIMEOUT)
    .execute()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "{DELIVERY_PROCESS_FAILURE_ERROR}: {description}: {}: {stderr}",
            output.status
        );
    }
    Ok(())
}
