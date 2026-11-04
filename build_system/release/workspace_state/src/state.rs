use crate::constants::{
    GIT_CONFIG_ARGUMENT, GIT_EXECUTABLE, GIT_HEAD_ARGUMENTS, GIT_INDEX_FLAGS_ARGUMENTS,
    GIT_INDEX_RECORD_SEPARATOR, GIT_INDEX_TAG_SEPARATOR, GIT_NO_FSMONITOR_CONFIG,
    GIT_NO_REPLACE_ARGUMENT, GIT_NO_UNTRACKED_CACHE_CONFIG, GIT_OBJECT_FORMAT_ARGUMENTS,
    GIT_ROOT_ARGUMENTS, GIT_SKIP_WORKTREE_TAG, GIT_STATUS_ARGUMENTS, SNAPSHOT_COMMAND_TIMEOUT,
    SUPPORTED_OBJECT_FORMATS, WORKSPACE_CHANGED_ERROR, WORKSPACE_DIRECTORY_ERROR,
    WORKSPACE_DIRTY_ERROR, WORKSPACE_INDEX_FLAGS_ERROR, WORKSPACE_INDEX_OUTPUT_ERROR,
    WORKSPACE_ROOT_ERROR,
};
use anyhow::{Context, Result, ensure};
use std::fs;
use std::path::{Path, PathBuf};
use yolo_release_filesystem::DirectoryHandle;
use yolo_release_foundation::validation::validate_revision;
use yolo_release_process_execution::{
    ProcessCommand,
    constants::{EMPTY_HOME, SYSTEM_TEMPORARY_DIRECTORY},
};

pub struct WorkspaceState {
    root: PathBuf,
    root_handle: DirectoryHandle,
    revision: String,
    object_format: String,
}

impl WorkspaceState {
    pub fn validate(workspace: &Path) -> Result<Self> {
        let root = fs::canonicalize(workspace)
            .with_context(|| format!("resolve release workspace {}", workspace.display()))?;
        ensure!(root.is_dir(), WORKSPACE_DIRECTORY_ERROR);
        let root_handle = DirectoryHandle::open(&root)?;
        let reported = fs::canonicalize(git_value(&root_handle, GIT_ROOT_ARGUMENTS)?)?;
        ensure!(root == reported, WORKSPACE_ROOT_ERROR);
        let revision = git_value(&root_handle, GIT_HEAD_ARGUMENTS)?;
        validate_revision(&revision)?;
        require_clean(&root_handle)?;
        let object_format = git_value(&root_handle, GIT_OBJECT_FORMAT_ARGUMENTS)?;
        ensure!(
            SUPPORTED_OBJECT_FORMATS.contains(&object_format.as_str()),
            "unsupported Git object format"
        );
        Ok(Self {
            root,
            root_handle,
            revision,
            object_format,
        })
    }

    pub fn ensure_unchanged(&self) -> Result<()> {
        self.verify_root_identity()?;
        ensure!(
            git_value(&self.root_handle, GIT_HEAD_ARGUMENTS)? == self.revision,
            WORKSPACE_CHANGED_ERROR
        );
        require_clean(&self.root_handle)
    }

    pub fn verify_root_identity(&self) -> Result<()> {
        self.root_handle.verify_path()
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    #[cfg(test)]
    pub fn root_handle(&self) -> &DirectoryHandle {
        &self.root_handle
    }

    pub fn revision(&self) -> &str {
        &self.revision
    }

    pub fn object_format(&self) -> &str {
        &self.object_format
    }
}

fn require_clean(workspace: &DirectoryHandle) -> Result<()> {
    reject_hidden_index_flags(workspace)?;
    ensure!(
        git_value(workspace, GIT_STATUS_ARGUMENTS)?.is_empty(),
        WORKSPACE_DIRTY_ERROR
    );
    reject_hidden_index_flags(workspace)?;
    Ok(())
}

fn reject_hidden_index_flags(workspace: &DirectoryHandle) -> Result<()> {
    for record in git_output(workspace, GIT_INDEX_FLAGS_ARGUMENTS)?
        .split(|byte| *byte == GIT_INDEX_RECORD_SEPARATOR)
        .filter(|record| !record.is_empty())
    {
        ensure!(
            record.get(1) == Some(&GIT_INDEX_TAG_SEPARATOR),
            WORKSPACE_INDEX_OUTPUT_ERROR
        );
        let tag = record[0];
        ensure!(
            tag != GIT_SKIP_WORKTREE_TAG && !tag.is_ascii_lowercase(),
            WORKSPACE_INDEX_FLAGS_ERROR
        );
    }
    Ok(())
}

fn git_value(workspace: &DirectoryHandle, arguments: &[&str]) -> Result<String> {
    Ok(String::from_utf8(git_output(workspace, arguments)?)
        .context("Git workspace output is not UTF-8")?
        .trim()
        .to_owned())
}

fn git_output(workspace: &DirectoryHandle, arguments: &[&str]) -> Result<Vec<u8>> {
    let home = DirectoryHandle::open(Path::new(EMPTY_HOME))?;
    let temporary = DirectoryHandle::open(Path::new(SYSTEM_TEMPORARY_DIRECTORY))?;
    let arguments = [
        &[
            GIT_NO_REPLACE_ARGUMENT,
            GIT_CONFIG_ARGUMENT,
            GIT_NO_FSMONITOR_CONFIG,
        ][..],
        &[GIT_CONFIG_ARGUMENT, GIT_NO_UNTRACKED_CACHE_CONFIG],
        arguments,
    ]
    .concat()
    .into_iter()
    .map(str::to_owned)
    .collect();
    let output = ProcessCommand::new(
        GIT_EXECUTABLE,
        arguments,
        workspace,
        &home,
        &temporary,
        "isolated Git workspace query",
    )
    .with_timeout(SNAPSHOT_COMMAND_TIMEOUT)
    .execute()?;
    ensure!(
        output.status.success(),
        "Git workspace query failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    Ok(output.stdout)
}
