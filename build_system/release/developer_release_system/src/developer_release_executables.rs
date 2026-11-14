use crate::delivery_files::ProtectedFile;
use crate::developer_release_command::DeveloperReleaseCommandRunner;
use crate::developer_release_constants::*;
use crate::developer_release_files::copy_writable;
use anyhow::{Context, Result, ensure};
use std::ffi::CString;
use std::fs::File;
use std::os::fd::FromRawFd;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path};
use yolo_release_archive_audit::model::ZipInventory;
use yolo_release_contracts::DELIVERY_OPEN_FAILURE;
use yolo_release_contracts::EXPECTED_FILE_LINKS;
use yolo_release_filesystem::{DirectoryHandle, TemporaryDirectory};
use yolo_release_foundation::digest::opened_file_sha256;

pub struct ExecutableBaseline {
    digests: Vec<String>,
}

impl ExecutableBaseline {
    pub fn capture(
        root: &DirectoryHandle,
        inventory: &ZipInventory,
        runner: &dyn DeveloperReleaseCommandRunner,
    ) -> Result<Self> {
        Ok(Self {
            digests: normalized_digests(root, Some(inventory), runner)?,
        })
    }

    pub fn verify(
        &self,
        root: &DirectoryHandle,
        runner: &dyn DeveloperReleaseCommandRunner,
    ) -> Result<()> {
        ensure!(
            normalized_digests(root, None, runner)? == self.digests,
            DEVELOPER_RELEASE_EXECUTABLE_CHANGED_ERROR
        );
        Ok(())
    }
}

fn normalized_digests(
    root: &DirectoryHandle,
    inventory: Option<&ZipInventory>,
    runner: &dyn DeveloperReleaseCommandRunner,
) -> Result<Vec<String>> {
    root.verify_self()?;
    let temporary = TemporaryDirectory::create(DEVELOPER_RELEASE_BASELINE_PREFIX)?;
    let workspace = temporary.handle();
    let mut digests = Vec::with_capacity(SIGNED_ARCHIVE_PATHS.len());
    for (index, path) in SIGNED_ARCHIVE_PATHS.iter().enumerate() {
        let source = open_relative(root, path)?;
        if let Some(inventory) = inventory {
            validate_candidate_source(&source, path, inventory)?;
        }
        let name = format!("{NORMALIZED_EXECUTABLE_PREFIX}{index}");
        copy_writable(workspace, &name, &source)?;
        runner.remove_signature(workspace, &name)?;
        let normalized = ProtectedFile::seal_existing(workspace, &name)?;
        digests.push(opened_file_sha256(normalized.file())?);
    }
    root.verify_self()?;
    Ok(digests)
}

fn validate_candidate_source(source: &File, path: &str, inventory: &ZipInventory) -> Result<()> {
    let entry = inventory
        .entries
        .iter()
        .find(|entry| entry.path == path)
        .context(DEVELOPER_RELEASE_EXECUTABLE_CHANGED_ERROR)?;
    ensure!(
        !entry.is_directory && opened_file_sha256(source)? == entry.sha256,
        DEVELOPER_RELEASE_EXECUTABLE_CHANGED_ERROR
    );
    Ok(())
}

fn open_relative(root: &DirectoryHandle, relative: &str) -> Result<File> {
    let components = Path::new(relative)
        .components()
        .map(normal_component)
        .collect::<Result<Vec<_>>>()?;
    let (name, parents) = components
        .split_last()
        .context(DEVELOPER_RELEASE_EXECUTABLE_CHANGED_ERROR)?;
    let mut directory = root.clone();
    for parent in parents {
        directory = DirectoryHandle::open_at(&directory, parent)?;
    }
    let name = CString::new(*name)?;
    let descriptor = unsafe {
        libc::openat(
            directory.raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if descriptor == DELIVERY_OPEN_FAILURE {
        return Err(std::io::Error::last_os_error())
            .context(DEVELOPER_RELEASE_EXECUTABLE_CHANGED_ERROR);
    }
    let file = unsafe { File::from_raw_fd(descriptor) };
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file()
            && metadata.uid() == unsafe { libc::geteuid() }
            && metadata.nlink() == EXPECTED_FILE_LINKS,
        DEVELOPER_RELEASE_EXECUTABLE_CHANGED_ERROR
    );
    Ok(file)
}

fn normal_component(component: Component<'_>) -> Result<&str> {
    match component {
        Component::Normal(value) => value
            .to_str()
            .context(DEVELOPER_RELEASE_EXECUTABLE_CHANGED_ERROR),
        _ => anyhow::bail!(DEVELOPER_RELEASE_EXECUTABLE_CHANGED_ERROR),
    }
}
