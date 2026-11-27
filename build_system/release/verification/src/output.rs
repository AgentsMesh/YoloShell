use anyhow::{Context, Result, ensure};
use std::path::Path;
use yolo_release_contracts::{NO_FILE_FLAGS, PRIVATE_DIRECTORY_MODE, SEALED_DIRECTORY_MODE};
use yolo_release_filesystem::{DirectoryHandle, DirectorySeal};
use yolo_release_gate_system::{ensure_missing, validate_directory, validate_parent};

pub struct OutputTransaction {
    pub(crate) parent: DirectoryHandle,
    pub(crate) destination: String,
}

impl OutputTransaction {
    pub fn prepare(path: &Path) -> Result<Self> {
        let destination = path
            .file_name()
            .and_then(|value| value.to_str())
            .context("candidate output name must be UTF-8")?;
        ensure!(
            !destination.is_empty() && destination != "." && destination != "..",
            "candidate output name is invalid"
        );
        let parent_path = path.parent().context("candidate output has no parent")?;
        let parent =
            DirectoryHandle::open(parent_path).context("open candidate output parent directory")?;
        validate_parent(&parent)?;
        ensure_missing(&parent, destination)?;
        Ok(Self {
            parent,
            destination: destination.to_owned(),
        })
    }
}

pub struct OutputDirectory {
    pub(crate) directory: DirectoryHandle,
    parent: Option<DirectoryHandle>,
    name: Option<String>,
    seal: DirectorySeal,
}

impl OutputDirectory {
    pub fn open(path: &Path) -> Result<Self> {
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .context("candidate output name must be UTF-8")?;
        let parent_path = path.parent().context("candidate output has no parent")?;
        let parent =
            DirectoryHandle::open(parent_path).context("open candidate output parent directory")?;
        validate_parent(&parent)?;
        let directory =
            DirectoryHandle::open_at(&parent, name).context("open candidate output directory")?;
        Self::from_bound(parent, name.to_owned(), directory)
    }

    pub(crate) fn from_handle(directory: DirectoryHandle) -> Result<Self> {
        validate_directory(
            &directory,
            "staged candidate directory",
            PRIVATE_DIRECTORY_MODE,
            NO_FILE_FLAGS,
        )?;
        let seal = directory.seal()?;
        Ok(Self {
            directory,
            parent: None,
            name: None,
            seal,
        })
    }

    pub(crate) fn from_bound(
        parent: DirectoryHandle,
        name: String,
        directory: DirectoryHandle,
    ) -> Result<Self> {
        Self::from_bound_policy(
            parent,
            name,
            directory,
            SEALED_DIRECTORY_MODE,
            libc::UF_IMMUTABLE,
        )
    }

    pub(crate) fn from_uncommitted(
        parent: DirectoryHandle,
        name: String,
        directory: DirectoryHandle,
    ) -> Result<Self> {
        Self::from_bound_policy(
            parent,
            name,
            directory,
            PRIVATE_DIRECTORY_MODE,
            NO_FILE_FLAGS,
        )
    }

    fn from_bound_policy(
        parent: DirectoryHandle,
        name: String,
        directory: DirectoryHandle,
        mode: u32,
        flags: u32,
    ) -> Result<Self> {
        validate_parent(&parent)?;
        validate_directory(&directory, "candidate output directory", mode, flags)?;
        parent.verify_child(&name, &directory)?;
        let seal = directory.seal()?;
        Ok(Self {
            directory,
            parent: Some(parent),
            name: Some(name),
            seal,
        })
    }

    pub(crate) fn verify_namespace(&self) -> Result<()> {
        self.directory.verify_seal(&self.seal)?;
        if let (Some(parent), Some(name)) = (&self.parent, &self.name) {
            validate_parent(parent)?;
            parent.verify_child(name, &self.directory)?;
        }
        Ok(())
    }
}
