use anyhow::{Context, Result, ensure};
use std::path::Path;
use std::process::Output;
use yolo_release_contracts::GIT_DIRECTORY_NAME;
use yolo_release_foundation::validation::{
    GIT_SHA1_HEX_LENGTH, GIT_SHA256_HEX_LENGTH, validate_revision,
};
use yolo_release_process_execution::ProcessCommand;
use yolo_release_workspace_state::constants::{
    GIT_DIRECTORY_PREFIX, GIT_PACK_EXTENSION, GIT_PACK_FILE_PREFIX, GIT_SHA1_FORMAT,
    SNAPSHOT_COMMAND_TIMEOUT,
};

pub(crate) fn private_arguments<'a>(
    operation: &[&str],
    trailing: impl IntoIterator<Item = &'a String>,
) -> Vec<String> {
    std::iter::once(format!("{GIT_DIRECTORY_PREFIX}{GIT_DIRECTORY_NAME}"))
        .chain(operation.iter().map(|value| (*value).to_owned()))
        .chain(trailing.into_iter().cloned())
        .collect()
}

pub(crate) fn validate_pack_digest(digest: &str, format: &str) -> Result<()> {
    validate_revision(digest)?;
    let expected = if format == GIT_SHA1_FORMAT {
        GIT_SHA1_HEX_LENGTH
    } else {
        GIT_SHA256_HEX_LENGTH
    };
    ensure!(
        digest.len() == expected,
        "private Git pack digest length mismatch"
    );
    Ok(())
}

pub(crate) fn pack_digest_from_path(path: &Path) -> Result<&str> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .context("private pack name")?;
    name.strip_prefix(GIT_PACK_FILE_PREFIX)
        .and_then(|value| value.strip_suffix(GIT_PACK_EXTENSION))
        .context("private pack name mismatch")
}

pub(crate) fn execute(command: ProcessCommand) -> Result<()> {
    execute_output(command).map(|_| ())
}

pub(crate) fn execute_output(command: ProcessCommand) -> Result<Output> {
    let output = command.with_timeout(SNAPSHOT_COMMAND_TIMEOUT).execute()?;
    ensure!(
        output.status.success(),
        "private Git command failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    Ok(output)
}
