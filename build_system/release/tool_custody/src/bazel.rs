use crate::identity::verify_private_path;
use crate::support::{candidate_path, checked_version, utf8};
use anyhow::{Context, Result, ensure};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Seek, SeekFrom};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use yolo_release_contracts::{
    BAZEL_LAUNCHER_NAME, BAZEL_LAUNCHER_PREFIX, BAZEL_VERSION_ARGUMENT, EXECUTABLE_PERMISSION_MASK,
    MAX_BAZEL_LAUNCHER_BYTES, PINNED_BAZEL_SHA256, PINNED_BAZEL_VERSION, PRIVATE_EXECUTABLE_MODE,
    PRIVATE_FILE_MODE,
};
use yolo_release_evidence_model::BazelLauncherEvidence;
use yolo_release_filesystem::{DirectoryHandle, DirectorySeal, TemporaryDirectory};
use yolo_release_foundation::digest::{copy_opened_bounded, opened_file_sha256};
use yolo_release_process_execution::constants::{BAZEL_REAL_ENVIRONMENT, HOME_ENVIRONMENT};

#[derive(Debug)]
pub struct TrustedBazel {
    _root: Option<TemporaryDirectory>,
    root_handle: Option<DirectoryHandle>,
    root_seal: Option<DirectorySeal>,
    pinned_file: Option<File>,
    pinned_identity: Option<(u64, u64)>,
    path: PathBuf,
    evidence: BazelLauncherEvidence,
}

impl TrustedBazel {
    pub fn resolve() -> Result<Self> {
        let candidate = candidate_path(
            env::var_os(BAZEL_REAL_ENVIRONMENT),
            env::var_os(HOME_ENVIRONMENT),
        )?;
        Self::validate(&candidate)
    }

    fn validate(candidate: &Path) -> Result<Self> {
        Self::validate_identity(
            candidate,
            PINNED_BAZEL_SHA256,
            BAZEL_VERSION_ARGUMENT,
            PINNED_BAZEL_VERSION,
        )
    }

    fn validate_identity(
        candidate: &Path,
        expected_digest: &str,
        version_argument: &str,
        expected_version: &str,
    ) -> Result<Self> {
        let canonical = fs::canonicalize(candidate)
            .with_context(|| format!("resolve pinned Bazel launcher {}", candidate.display()))?;
        let mut source = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&canonical)
            .with_context(|| format!("open pinned Bazel launcher {}", canonical.display()))?;
        let metadata = source.metadata()?;
        ensure!(metadata.is_file(), "pinned Bazel launcher must be a file");
        ensure!(
            metadata.permissions().mode() & EXECUTABLE_PERMISSION_MASK != 0,
            "pinned Bazel launcher must be executable"
        );
        let source_before = opened_file_sha256(&source)?;
        ensure!(
            source_before == expected_digest,
            "pinned Bazel digest mismatch"
        );
        let mut root = TemporaryDirectory::create(BAZEL_LAUNCHER_PREFIX)?;
        let root_handle = root.handle().clone();
        let execution_path = root.path().join(BAZEL_LAUNCHER_NAME);
        let mut executable = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(PRIVATE_FILE_MODE)
            .open(&execution_path)?;
        source.seek(SeekFrom::Start(0))?;
        copy_opened_bounded(
            &source,
            &mut executable,
            MAX_BAZEL_LAUNCHER_BYTES,
            "pinned Bazel launcher",
        )?;
        executable
            .sync_all()
            .context("sync private Bazel launcher")?;
        fs::set_permissions(
            &execution_path,
            fs::Permissions::from_mode(PRIVATE_EXECUTABLE_MODE),
        )?;
        let copied = opened_file_sha256(&executable)?;
        let source_after = opened_file_sha256(&source)?;
        ensure!(
            source_before == copied && copied == source_after,
            "pinned Bazel changed while it was copied"
        );
        drop(executable);
        let pinned_file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&execution_path)
            .with_context(|| format!("pin private Bazel launcher {}", execution_path.display()))?;
        let pinned_metadata = pinned_file.metadata()?;
        let pinned_identity = (pinned_metadata.dev(), pinned_metadata.ino());
        ensure!(
            opened_file_sha256(&pinned_file)? == copied,
            "private Bazel changed before version validation"
        );
        verify_private_path(
            &root_handle,
            &execution_path,
            &pinned_file,
            pinned_identity,
            &copied,
        )?;
        let root_seal = root.seal()?;
        let version = checked_version(
            &execution_path,
            &root_handle,
            version_argument,
            expected_version,
        )?;
        root_handle.verify_seal(&root_seal)?;
        verify_private_path(
            &root_handle,
            &execution_path,
            &pinned_file,
            pinned_identity,
            &copied,
        )?;
        let evidence = BazelLauncherEvidence {
            canonical_path: utf8(&canonical)?,
            execution_path: utf8(&execution_path)?,
            sha256: copied,
            version,
        };
        let launcher = Self {
            _root: Some(root),
            root_handle: Some(root_handle),
            root_seal: Some(root_seal),
            pinned_file: Some(pinned_file),
            pinned_identity: Some(pinned_identity),
            path: execution_path,
            evidence,
        };
        Ok(launcher)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn resource_directory(&self) -> Option<&DirectoryHandle> {
        self.root_handle.as_ref()
    }

    pub fn evidence(&self) -> BazelLauncherEvidence {
        self.evidence.clone()
    }

    pub fn ensure_unchanged(&self) -> Result<()> {
        if let (Some(root), Some(seal), Some(file), Some(identity)) = (
            &self.root_handle,
            &self.root_seal,
            &self.pinned_file,
            self.pinned_identity,
        ) {
            root.verify_seal(seal)?;
            verify_private_path(root, &self.path, file, identity, &self.evidence.sha256)?;
        }
        Ok(())
    }
}

#[cfg(any(test, feature = "yolo-test-support"))]
#[path = "../test_support/bazel.rs"]
mod test_support;
