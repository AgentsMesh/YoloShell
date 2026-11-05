mod constants;
mod held;
use anyhow::{Result, ensure};
use constants::*;
use held::Held;
use std::sync::Arc;
use yolo_distribution_bundle::{
    ArtifactRole, BIN_DIRECTORY, BundleDescriptor, BundleSource, MANIFEST_FILE_NAME,
    MAX_ARTIFACT_BYTES,
};

/// Retained local files and validated immutable byte snapshots. Not serializable authority.
pub struct VerifiedBundle {
    source: BundleSource,
    held: Vec<Held>,
    runtime: Arc<[u8]>,
}
impl VerifiedBundle {
    pub fn open(source: &BundleSource) -> Result<Self> {
        let root = Held::open(source.root(), true, 0)?;
        let bin = Held::open(&source.root().join(BIN_DIRECTORY), true, 0)?;
        let manifest = Held::open(
            &source.root().join(MANIFEST_FILE_NAME),
            false,
            MAX_MANIFEST_BYTES,
        )?;
        let descriptor: BundleDescriptor = serde_json::from_slice(&manifest.bytes()?)?;
        ensure!(&descriptor == source.descriptor(), MANIFEST_MISMATCH);
        let file = Held::open(
            &source.root().join(ArtifactRole::Runtime.relative_path()),
            false,
            MAX_ARTIFACT_BYTES,
        )?;
        ensure!(file.executable(), INVALID);
        let bytes = file.bytes()?;
        descriptor.verify_artifact(ArtifactRole::Runtime, &bytes)?;
        let verified = Self {
            source: source.clone(),
            held: vec![root, bin, manifest, file],
            runtime: Arc::from(bytes),
        };
        verified.verify()?;
        Ok(verified)
    }
    pub fn source(&self) -> &BundleSource {
        &self.source
    }
    pub fn verify(&self) -> Result<()> {
        for held in &self.held {
            held.verify()?;
        }
        Ok(())
    }
    pub fn payload(&self, role: ArtifactRole) -> Result<Arc<[u8]>> {
        self.verify()?;
        match role {
            ArtifactRole::Runtime => Ok(self.runtime.clone()),
        }
    }
}
