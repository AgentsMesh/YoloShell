use crate::{constants::*, file::HeldFile, inventory};
use anyhow::{Result, ensure};
use std::path::Path;
use yolo_distribution_bundle::{
    ArtifactRole, BIN_DIRECTORY, BundleDescriptor, MANIFEST_FILE_NAME, MAX_ARTIFACT_BYTES,
};
use yolo_paths::directory::PrivateDirectory;

pub(crate) struct VerifiedRelease {
    pub(crate) directory: PrivateDirectory,
    bin: PrivateDirectory,
    manifest: HeldFile,
    artifacts: Vec<HeldFile>,
    pub(crate) descriptor: BundleDescriptor,
}

impl VerifiedRelease {
    pub(crate) fn open(
        parent: &PrivateDirectory,
        path: &Path,
        expected: Option<&BundleDescriptor>,
    ) -> Result<Self> {
        parent.verify_path()?;
        let directory = parent.open_descendant(path)?;
        inventory::exact(&directory, &[MANIFEST_FILE_NAME, BIN_DIRECTORY])?;
        let bin = directory.open_descendant(&path.join(BIN_DIRECTORY))?;
        inventory::exact(&bin, &ArtifactRole::ALL.map(ArtifactRole::executable_name))?;
        let manifest = HeldFile::open(
            &directory,
            MANIFEST_FILE_NAME,
            MANIFEST_MODE,
            MAX_MANIFEST_BYTES,
        )?;
        let descriptor: BundleDescriptor =
            serde_json::from_slice(&manifest.read_manifest(&directory)?)?;
        ensure!(
            expected.is_none_or(|expected| *expected == descriptor),
            RELEASE_MISMATCH
        );
        let artifacts = ArtifactRole::ALL
            .into_iter()
            .map(|role| {
                let held = HeldFile::open(
                    &bin,
                    role.executable_name(),
                    EXECUTABLE_MODE,
                    MAX_ARTIFACT_BYTES,
                )?;
                held.verify_artifact(&bin, descriptor.artifact(role), descriptor.target())?;
                Ok(held)
            })
            .collect::<Result<Vec<_>>>()?;
        let release = Self {
            directory,
            bin,
            manifest,
            artifacts,
            descriptor,
        };
        release.verify()?;
        parent.verify_path()?;
        Ok(release)
    }

    pub(crate) fn verify(&self) -> Result<()> {
        inventory::exact(&self.directory, &[MANIFEST_FILE_NAME, BIN_DIRECTORY])?;
        inventory::exact(
            &self.bin,
            &ArtifactRole::ALL.map(ArtifactRole::executable_name),
        )?;
        self.manifest.verify(&self.directory)?;
        for artifact in &self.artifacts {
            artifact.verify(&self.bin)?;
        }
        Ok(())
    }

    pub(crate) fn sync(&self) -> Result<()> {
        self.verify()?;
        for artifact in &self.artifacts {
            artifact.sync()?;
        }
        self.manifest.sync()?;
        self.bin.sync()?;
        self.directory.sync()?;
        self.verify()
    }

    pub(crate) fn rebind(mut self, parent: &PrivateDirectory, path: &Path) -> Result<Self> {
        parent.verify_path()?;
        let directory = parent.open_descendant(path)?;
        ensure!(directory.same_identity(&self.directory), IDENTITY_CHANGED);
        let bin = directory.open_descendant(&path.join(BIN_DIRECTORY))?;
        ensure!(bin.same_identity(&self.bin), IDENTITY_CHANGED);
        self.directory = directory;
        self.bin = bin;
        self.verify()?;
        parent.verify_path()?;
        Ok(self)
    }
}
