use crate::identity::verify_private_path;
use crate::support::checked_version;
use anyhow::{Context, Result, ensure};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Seek, SeekFrom};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use yolo_release_contracts::*;
use yolo_release_filesystem::{DirectoryHandle, DirectorySeal, TemporaryDirectory};
use yolo_release_foundation::digest::{copy_opened_bounded, opened_file_sha256};

const CLIENT_DESCRIPTION: &str = "Docker client";
const COPY_CHANGED_ERROR: &str = "Docker client changed while it was copied";
const DIGEST_ERROR: &str = "Docker client digest mismatch";
const EXECUTABLE_ERROR: &str = "Docker client must be executable";
const OPEN_CONTEXT: &str = "open Docker client";
const RESOLVE_CONTEXT: &str = "resolve Docker client";
const TYPE_ERROR: &str = "Docker client must be a file";

pub struct DockerClient {
    _root: TemporaryDirectory,
    root: DirectoryHandle,
    seal: DirectorySeal,
    execution: PathBuf,
    canonical: PathBuf,
    file: File,
    identity: (u64, u64),
    sha256: String,
    version: String,
}

pub trait DockerClientTool {
    fn path(&self) -> &Path;
    fn resource_directory(&self) -> Option<&DirectoryHandle>;
    fn canonical_path(&self) -> &Path;
    fn sha256(&self) -> &str;
    fn version(&self) -> &str;
    fn ensure_unchanged(&self) -> Result<()>;
}

impl DockerClient {
    pub fn resolve() -> Result<Self> {
        let candidate = env::var_os(DOCKER_CLIENT_ENVIRONMENT)
            .with_context(|| format!("{DOCKER_CLIENT_ENVIRONMENT} is required"))?;
        Self::capture(
            Path::new(&candidate),
            DOCKER_CLIENT_SHA256,
            DOCKER_CLIENT_VERSION,
        )
    }

    fn capture(source_path: &Path, expected_sha256: &str, expected_version: &str) -> Result<Self> {
        let canonical = fs::canonicalize(source_path).context(RESOLVE_CONTEXT)?;
        let mut source = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&canonical)
            .context(OPEN_CONTEXT)?;
        let metadata = source.metadata()?;
        ensure!(metadata.is_file(), TYPE_ERROR);
        ensure!(
            metadata.permissions().mode() & EXECUTABLE_PERMISSION_MASK != 0,
            EXECUTABLE_ERROR
        );
        let sha256 = opened_file_sha256(&source)?;
        ensure!(sha256 == expected_sha256, DIGEST_ERROR);
        let mut root = TemporaryDirectory::create(DOCKER_CLIENT_PREFIX)?;
        let root_handle = root.handle().clone();
        let execution = root.path().join(DOCKER_CLIENT_NAME);
        let mut destination = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(PRIVATE_FILE_MODE)
            .open(&execution)?;
        source.seek(SeekFrom::Start(0))?;
        copy_opened_bounded(
            &source,
            &mut destination,
            MAX_DOCKER_CLIENT_BYTES,
            CLIENT_DESCRIPTION,
        )?;
        destination.sync_all()?;
        fs::set_permissions(
            &execution,
            fs::Permissions::from_mode(PRIVATE_EXECUTABLE_MODE),
        )?;
        ensure!(
            opened_file_sha256(&destination)? == sha256 && opened_file_sha256(&source)? == sha256,
            COPY_CHANGED_ERROR
        );
        drop(destination);
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&execution)?;
        let pinned = file.metadata()?;
        let identity = (pinned.dev(), pinned.ino());
        verify_private_path(&root_handle, &execution, &file, identity, &sha256)?;
        let seal = root.seal()?;
        let version = checked_version(
            &execution,
            &root_handle,
            DOCKER_VERSION_ARGUMENT,
            expected_version,
        )?;
        let client = Self {
            _root: root,
            root: root_handle,
            seal,
            execution,
            canonical,
            identity,
            file,
            sha256,
            version,
        };
        client.ensure_unchanged()?;
        Ok(client)
    }
}

impl DockerClientTool for DockerClient {
    fn resource_directory(&self) -> Option<&DirectoryHandle> {
        Some(&self.root)
    }

    fn path(&self) -> &Path {
        &self.execution
    }

    fn canonical_path(&self) -> &Path {
        &self.canonical
    }

    fn sha256(&self) -> &str {
        &self.sha256
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn ensure_unchanged(&self) -> Result<()> {
        self.root.verify_seal(&self.seal)?;
        verify_private_path(
            &self.root,
            &self.execution,
            &self.file,
            self.identity,
            &self.sha256,
        )
    }
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/docker_tests.rs"]
mod tests;

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/docker_cleanup_tests.rs"]
mod cleanup_tests;
