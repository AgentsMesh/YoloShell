use anyhow::{Context, Result, ensure};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Seek, SeekFrom};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use yolo_release_contracts::*;
use yolo_release_filesystem::{DirectoryHandle, DirectorySeal, TemporaryDirectory};
use yolo_release_foundation::digest::{copy_opened_bounded, opened_file_sha256};
use yolo_release_gate_system::{LINUX_AMD64_PROFILE, LINUX_ARM64_PROFILE, LinuxGateProfile};
use yolo_release_tool_custody::verify_private_path;

pub trait LinuxBazelTool {
    fn path(&self) -> &Path;
    fn resource_directory(&self) -> Option<&DirectoryHandle>;
    fn sha256(&self) -> &str;
    fn ensure_unchanged(&self) -> Result<()>;
}

pub struct LinuxBazelToolchains {
    amd64: PinnedLinuxBazel,
    arm64: PinnedLinuxBazel,
}

impl LinuxBazelToolchains {
    pub fn resolve() -> Result<Self> {
        Ok(Self {
            amd64: PinnedLinuxBazel::resolve(LINUX_AMD64_PROFILE)?,
            arm64: PinnedLinuxBazel::resolve(LINUX_ARM64_PROFILE)?,
        })
    }

    pub fn tool(&self, profile: LinuxGateProfile) -> &dyn LinuxBazelTool {
        if profile == LINUX_AMD64_PROFILE {
            &self.amd64
        } else {
            &self.arm64
        }
    }
}

struct PinnedLinuxBazel {
    _root: TemporaryDirectory,
    root: DirectoryHandle,
    seal: DirectorySeal,
    file: File,
    identity: (u64, u64),
    path: PathBuf,
    sha256: String,
}

impl PinnedLinuxBazel {
    fn resolve(profile: LinuxGateProfile) -> Result<Self> {
        let candidate = env::var_os(profile.bazel_environment)
            .with_context(|| format!("{} is required", profile.bazel_environment))?;
        Self::capture(Path::new(&candidate), profile.bazel_sha256)
    }

    fn capture(candidate: &Path, expected: &str) -> Result<Self> {
        Self::capture_with(candidate, expected, |_| Ok(()))
    }

    fn capture_with(
        candidate: &Path,
        expected: &str,
        after_seal: impl FnOnce(&Self) -> Result<()>,
    ) -> Result<Self> {
        let canonical = fs::canonicalize(candidate)
            .with_context(|| format!("resolve Linux Bazel {}", candidate.display()))?;
        let mut source = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&canonical)?;
        let metadata = source.metadata()?;
        ensure!(metadata.is_file(), "Linux Bazel must be a regular file");
        ensure!(
            metadata.permissions().mode() & EXECUTABLE_PERMISSION_MASK != 0,
            "Linux Bazel must be executable"
        );
        let before = opened_file_sha256(&source)?;
        ensure!(before == expected, "Linux Bazel digest mismatch");
        let mut root = TemporaryDirectory::create(LINUX_BAZEL_PREFIX)?;
        let root_handle = root.handle().clone();
        let path = root.path().join(LINUX_BAZEL_NAME);
        let mut destination = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(PRIVATE_FILE_MODE)
            .open(&path)?;
        source.seek(SeekFrom::Start(0))?;
        copy_opened_bounded(
            &source,
            &mut destination,
            MAX_BAZEL_LAUNCHER_BYTES,
            "Linux Bazel",
        )?;
        destination.sync_all()?;
        fs::set_permissions(&path, fs::Permissions::from_mode(PRIVATE_EXECUTABLE_MODE))?;
        ensure!(
            opened_file_sha256(&destination)? == before,
            "copied Linux Bazel changed"
        );
        ensure!(
            opened_file_sha256(&source)? == before,
            "source Linux Bazel changed"
        );
        drop(destination);
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&path)?;
        let pinned = file.metadata()?;
        let identity = (pinned.dev(), pinned.ino());
        verify_private_path(&root_handle, &path, &file, identity, &before)?;
        let seal = root.seal()?;
        let pinned = Self {
            _root: root,
            root: root_handle,
            seal,
            file,
            identity,
            path,
            sha256: before,
        };
        after_seal(&pinned)?;
        pinned.ensure_unchanged()?;
        Ok(pinned)
    }
}

impl LinuxBazelTool for PinnedLinuxBazel {
    fn resource_directory(&self) -> Option<&DirectoryHandle> {
        Some(&self.root)
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn sha256(&self) -> &str {
        &self.sha256
    }

    fn ensure_unchanged(&self) -> Result<()> {
        self.root.verify_seal(&self.seal)?;
        verify_private_path(
            &self.root,
            &self.path,
            &self.file,
            self.identity,
            &self.sha256,
        )
    }
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/linux_bazel_tests.rs"]
mod tests;
