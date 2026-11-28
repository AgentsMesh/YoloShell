use crate::{Selection, constants::*, inventory, release::VerifiedRelease};
use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use yolo_distribution_bundle::{ArtifactRole, BundleDescriptor, RELEASES_DIRECTORY};
use yolo_paths::{
    coordination::CoordinationLock, directory::PrivateDirectory, identity::FileIdentity,
    namespace_lease::NamespaceLease,
};

pub struct ReleaseStore {
    pub(crate) root: PrivateDirectory,
    pub(crate) releases: PrivateDirectory,
    pub(crate) namespace: NamespaceLease,
}

#[derive(Clone, Debug)]
pub struct InstalledBundle {
    pub(crate) selection: Selection,
    pub(crate) descriptor: BundleDescriptor,
    pub(crate) directory: PathBuf,
}

impl InstalledBundle {
    pub fn selection(&self) -> &Selection {
        &self.selection
    }
    pub fn descriptor(&self) -> &BundleDescriptor {
        &self.descriptor
    }
    pub fn release_directory(&self) -> &Path {
        &self.directory
    }
    pub fn executable(&self, role: ArtifactRole) -> PathBuf {
        self.directory.join(role.relative_path())
    }
}

impl ReleaseStore {
    pub fn open(path: &Path) -> Result<Self> {
        let original = PrivateDirectory::open_or_create_strict(path)?;
        let canonical = std::fs::canonicalize(path)?;
        let root = PrivateDirectory::open(&canonical)?;
        ensure!(root.same_identity(&original), IDENTITY_CHANGED);
        original.verify_path()?;
        let releases =
            PrivateDirectory::open_or_create_strict(&canonical.join(RELEASES_DIRECTORY))?;
        root.verify_path()?;
        let namespace = NamespaceLease::from_directories(vec![
            (ROOT_NAMESPACE, root.try_clone()?),
            (RELEASES_NAMESPACE, releases.try_clone()?),
        ])?;
        Ok(Self {
            root,
            releases,
            namespace,
        })
    }

    pub fn root(&self) -> &Path {
        self.root.path()
    }

    pub fn stage_path(&self, stage: Uuid) -> Result<PathBuf> {
        ensure!(!stage.is_nil(), INVALID_OPERATION);
        Ok(self.root.path().join(format!("{STAGE_PREFIX}{stage}")))
    }

    /// Observation verifies the selected release. An I/O error never proves no publication.
    pub fn inspect(&self) -> Result<Option<Selection>> {
        Ok(self
            .inspect_installed()?
            .map(|installed| installed.selection))
    }

    pub fn inspect_installed(&self) -> Result<Option<InstalledBundle>> {
        let lock = self.lock()?;
        let installed = self.inspect_existing()?;
        self.verify(&lock)?;
        Ok(installed)
    }

    /// Finds a complete release after a failed rename/sync acknowledgement, without selecting it.
    pub fn inspect_release(&self, descriptor: &BundleDescriptor) -> Result<Option<PathBuf>> {
        let lock = self.lock()?;
        let result = if inventory::exists(&self.releases, descriptor.build_id())? {
            let path = descriptor.release_directory(self.root.path());
            VerifiedRelease::open(&self.releases, &path, Some(descriptor))?.verify()?;
            Some(path)
        } else {
            None
        };
        self.verify(&lock)?;
        Ok(result)
    }

    pub(crate) fn selected_release(&self, selection: &Selection) -> Result<VerifiedRelease> {
        let path = self.releases.path().join(selection.build_id());
        let release = VerifiedRelease::open(&self.releases, &path, None)?;
        ensure!(
            release.descriptor.build_id() == selection.build_id(),
            RELEASE_MISMATCH
        );
        Ok(release)
    }

    pub(crate) fn read_selection(&self) -> Result<Option<(Selection, FileIdentity)>> {
        let path = self.root.path().join(CURRENT_FILE_NAME);
        if self.namespace.file_is_missing(&path)? {
            return Ok(None);
        }
        self.namespace.read_json_with_identity(&path).map(Some)
    }

    pub(crate) fn verify_selection(
        &self,
        expected: &Option<(Selection, FileIdentity)>,
    ) -> Result<()> {
        ensure!(self.read_selection()? == *expected, CONFLICT);
        Ok(())
    }

    pub(crate) fn lock(&self) -> Result<CoordinationLock> {
        self.namespace.verify()?;
        let lock =
            CoordinationLock::try_acquire(&self.root.path().join(LOCK_FILE_NAME), LOCK_BYTES)?
                .context(BUSY)?;
        self.verify(&lock)?;
        Ok(lock)
    }

    pub(crate) fn verify(&self, lock: &CoordinationLock) -> Result<()> {
        self.namespace.verify()?;
        lock.verify()
    }
}
