use crate::constants::{
    DEFAULT_PROCESS_OUTPUT_BYTES, MAX_PROCESS_INPUT_BYTES, RELEASE_COMMAND_TIMEOUT,
};
use crate::posix_supervisor::supervised_process;
use anyhow::Result;
use std::fs::File;
use std::path::PathBuf;
use std::process::Output;
use std::sync::Arc;
use std::time::Duration;
use yolo_release_filesystem::DirectoryHandle;

#[derive(Clone, Debug)]
pub(crate) struct DescriptorBinding {
    pub file: Arc<File>,
    pub child_descriptor: libc::c_int,
}

#[derive(Clone, Debug)]
pub struct ProcessCommand {
    pub(crate) executable: PathBuf,
    pub(crate) arguments: Vec<String>,
    pub(crate) workspace: DirectoryHandle,
    pub(crate) home: DirectoryHandle,
    pub(crate) temporary_directory: DirectoryHandle,
    pub(crate) resource_directories: Vec<DirectoryHandle>,
    pub(crate) inherited: Vec<DescriptorBinding>,
    pub(crate) input: Option<Vec<u8>>,
    pub(crate) output_limit: usize,
    pub(crate) timeout: Duration,
    pub(crate) description: String,
}

impl ProcessCommand {
    pub fn new(
        executable: impl Into<PathBuf>,
        arguments: Vec<String>,
        workspace: &DirectoryHandle,
        home: &DirectoryHandle,
        temporary_directory: &DirectoryHandle,
        description: impl Into<String>,
    ) -> Self {
        Self {
            executable: executable.into(),
            arguments,
            workspace: workspace.clone(),
            home: home.clone(),
            temporary_directory: temporary_directory.clone(),
            resource_directories: Vec::new(),
            inherited: Vec::new(),
            input: None,
            output_limit: DEFAULT_PROCESS_OUTPUT_BYTES,
            timeout: RELEASE_COMMAND_TIMEOUT,
            description: description.into(),
        }
    }

    pub fn with_input(mut self, input: Vec<u8>) -> Self {
        self.input = Some(input);
        self
    }

    /// Retain additional owned directories used by this command, such as its tool root.
    pub fn retain_directories<'a>(
        mut self,
        directories: impl IntoIterator<Item = &'a DirectoryHandle>,
    ) -> Self {
        self.resource_directories
            .extend(directories.into_iter().cloned());
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_output_limit(mut self, output_limit: usize) -> Self {
        self.output_limit = output_limit;
        self
    }

    pub fn inherit_as(mut self, file: Arc<File>, child_descriptor: libc::c_int) -> Self {
        self.inherited.push(DescriptorBinding {
            file,
            child_descriptor,
        });
        self
    }

    pub fn evidence(&self) -> Vec<String> {
        std::iter::once(self.executable.display().to_string())
            .chain(self.arguments.clone())
            .collect()
    }

    pub fn execute(&self) -> Result<Output> {
        self.workspace.verify_self()?;
        self.home.verify_self()?;
        self.temporary_directory.verify_self()?;
        if self
            .input
            .as_ref()
            .is_some_and(|value| value.len() > MAX_PROCESS_INPUT_BYTES)
        {
            anyhow::bail!("process input exceeded the bounded limit");
        }
        if self.output_limit == 0 {
            anyhow::bail!("process output limit must be positive");
        }
        supervised_process(self)
    }
}
