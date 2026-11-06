use anyhow::{Context, Result, ensure};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use yolo_release_contracts::{BAZEL_VERSION_TIMEOUT, PINNED_BAZEL_CACHE_PATH};
use yolo_release_filesystem::DirectoryHandle;
use yolo_release_process_execution::{
    ProcessCommand,
    constants::{EMPTY_HOME, SYSTEM_TEMPORARY_DIRECTORY},
};

const VERSION_COMMAND_DESCRIPTION: &str = "release tool version";
const VERSION_FAILURE_PREFIX: &str = "release tool version check failed";
const VERSION_MISMATCH: &str = "release tool version mismatch";
const VERSION_UTF8_CONTEXT: &str = "release tool version is not UTF-8";

pub fn checked_version(
    path: &Path,
    resource_directory: &DirectoryHandle,
    argument: &str,
    expected: &str,
) -> Result<String> {
    let workspace = DirectoryHandle::open(Path::new(SYSTEM_TEMPORARY_DIRECTORY))?;
    let home = DirectoryHandle::open(Path::new(EMPTY_HOME))?;
    let output = ProcessCommand::new(
        path,
        vec![argument.to_owned()],
        &workspace,
        &home,
        &workspace,
        VERSION_COMMAND_DESCRIPTION,
    )
    .retain_directories([resource_directory])
    .with_timeout(BAZEL_VERSION_TIMEOUT)
    .execute()?;
    ensure!(
        output.status.success(),
        "{} ({:?}): {}",
        VERSION_FAILURE_PREFIX,
        output.status,
        String::from_utf8_lossy(&output.stderr).trim()
    );
    let version = String::from_utf8(output.stdout).context(VERSION_UTF8_CONTEXT)?;
    ensure!(
        version.strip_suffix('\n') == Some(expected),
        VERSION_MISMATCH
    );
    Ok(expected.to_owned())
}

pub fn candidate_path(bazel_real: Option<OsString>, home: Option<OsString>) -> Result<PathBuf> {
    if let Some(path) = bazel_real.filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    let home = home.context("HOME is required to resolve pinned Bazel")?;
    Ok(PathBuf::from(home).join(PINNED_BAZEL_CACHE_PATH))
}

pub(crate) fn utf8(path: &Path) -> Result<String> {
    Ok(path
        .to_str()
        .context("pinned Bazel path must be UTF-8")?
        .to_owned())
}
