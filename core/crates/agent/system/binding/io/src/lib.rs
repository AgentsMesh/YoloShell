use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use yolo_agent_adapter_contract::constants::message;
use yolo_agent_contract::AgentBinding;
use yolo_paths::identity::FileIdentity;
use yolo_paths::namespace_lease::NamespaceLease;

#[derive(Debug, Clone)]
pub struct AgentBindingFile {
    path: PathBuf,
}

impl AgentBindingFile {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn read(&self) -> Result<Option<AgentBinding>> {
        optional_binding(yolo_paths::read_json(&self.path))
    }

    pub fn write(&self, binding: &AgentBinding) -> Result<()> {
        yolo_paths::atomic_write_json(&self.path, binding).context(message::PERSIST)
    }

    pub fn clear(&self) -> Result<()> {
        yolo_paths::remove_private_file(&self.path).context(message::REMOVE)
    }

    pub fn read_in_namespace(&self, namespace: &NamespaceLease) -> Result<Option<AgentBinding>> {
        optional_binding(namespace.read_json(&self.path))
    }

    pub fn read_with_identity_in_namespace(
        &self,
        namespace: &NamespaceLease,
    ) -> Result<Option<(AgentBinding, FileIdentity)>> {
        optional_binding(namespace.read_json_with_identity(&self.path))
    }

    pub fn persist_reconciled_in_namespace(
        &self,
        namespace: &NamespaceLease,
        expected: Option<FileIdentity>,
        binding: &AgentBinding,
        commit_fence: impl FnMut() -> Result<()>,
    ) -> Result<()> {
        namespace
            .write_json_expected_with_commit_fence(&self.path, expected, binding, commit_fence)
            .map(|_| ())
            .context(message::PERSIST)
    }
}

fn optional_binding<T>(result: Result<T>) -> Result<Option<T>> {
    match result {
        Ok(binding) => Ok(Some(binding)),
        Err(error) if is_missing(&error) => Ok(None),
        Err(error) if error.downcast_ref::<serde_json::Error>().is_some() => {
            Err(error).context(message::DECODE)
        }
        Err(error) => Err(error).context(message::READ),
    }
}

fn is_missing(error: &anyhow::Error) -> bool {
    error
        .chain()
        .find_map(|source| source.downcast_ref::<std::io::Error>())
        .is_some_and(|source| source.kind() == std::io::ErrorKind::NotFound)
}
