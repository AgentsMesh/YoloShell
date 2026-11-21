use super::RemoteError;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use uuid::Uuid;
use yolo_distribution_bundle::{ArtifactRole, BundleDescriptor, MANIFEST_FILE_NAME};
use yolo_distribution_publication_protocol::{
    MAX_INSTALL_PATH_BYTES, MAX_PUBLICATION_BYTES, STAGE_PREFIX,
};

const INVALID_STAGE: &str =
    "bundle stage requires a bounded absolute installation root and non-nil UUID";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BundleExecutableLocation {
    Staged,
    Published,
}

#[derive(Clone)]
pub struct BundlePayloads {
    pub runtime: Arc<[u8]>,
}

impl BundlePayloads {
    pub fn artifact(&self, role: ArtifactRole) -> &[u8] {
        match role {
            ArtifactRole::Runtime => &self.runtime,
        }
    }
}

#[derive(Clone)]
pub struct BundleStageRequest {
    root: PathBuf,
    stage: Uuid,
    descriptor: BundleDescriptor,
    payloads: BundlePayloads,
    manifest: Arc<[u8]>,
}

impl BundleStageRequest {
    pub fn new(
        root: PathBuf,
        stage: Uuid,
        descriptor: BundleDescriptor,
        payloads: BundlePayloads,
    ) -> Result<Self, RemoteError> {
        validate_install_root(&root)?;
        if stage.is_nil() {
            return Err(RemoteError::InvalidRuntime(INVALID_STAGE.into()));
        }
        for role in ArtifactRole::ALL {
            descriptor
                .verify_artifact(role, payloads.artifact(role))
                .map_err(|error| RemoteError::InvalidArtifact(error.to_string()))?;
        }
        let manifest = serde_json::to_vec(&descriptor)
            .map_err(|error| RemoteError::InvalidArtifact(error.to_string()))?;
        if manifest.len() > MAX_PUBLICATION_BYTES {
            return Err(RemoteError::LimitExceeded);
        }
        let result = Self {
            root,
            stage,
            descriptor,
            payloads,
            manifest: manifest.into(),
        };
        for role in ArtifactRole::ALL {
            validate_path(&result.executable(role))?;
        }
        validate_path(&result.stage_directory().join(MANIFEST_FILE_NAME))?;
        Ok(result)
    }
    pub fn install_root(&self) -> &Path {
        &self.root
    }
    pub fn stage_id(&self) -> Uuid {
        self.stage
    }
    pub fn descriptor(&self) -> &BundleDescriptor {
        &self.descriptor
    }
    pub fn stage_directory(&self) -> PathBuf {
        self.root.join(format!("{STAGE_PREFIX}{}", self.stage))
    }
    pub fn executable(&self, role: ArtifactRole) -> PathBuf {
        self.stage_directory().join(role.relative_path())
    }
    pub fn payload(&self, role: ArtifactRole) -> &[u8] {
        self.payloads.artifact(role)
    }
    pub(crate) fn manifest(&self) -> &[u8] {
        &self.manifest
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BundleStageResult {
    root: PathBuf,
    stage: Uuid,
    directory: PathBuf,
    descriptor: BundleDescriptor,
}

impl BundleStageResult {
    /// Derive response metadata after an adapter has verified the staged files.
    pub fn from_request(request: &BundleStageRequest) -> Self {
        Self {
            root: request.root.clone(),
            stage: request.stage,
            directory: request.stage_directory(),
            descriptor: request.descriptor.clone(),
        }
    }
    pub fn install_root(&self) -> &Path {
        &self.root
    }
    pub fn stage_id(&self) -> Uuid {
        self.stage
    }
    pub fn stage_directory(&self) -> &Path {
        &self.directory
    }
    pub fn executable(&self, role: ArtifactRole) -> PathBuf {
        self.directory.join(role.relative_path())
    }
    pub fn descriptor(&self) -> &BundleDescriptor {
        &self.descriptor
    }
}

pub(crate) fn validate_install_root(root: &Path) -> Result<(), RemoteError> {
    yolo_distribution_publication_protocol::validate_install_root(root)
        .map_err(|error| RemoteError::InvalidRuntime(error.to_string()))
}

fn validate_path(path: &Path) -> Result<(), RemoteError> {
    if path
        .to_str()
        .is_none_or(|text| text.len() > MAX_INSTALL_PATH_BYTES || text.contains('\0'))
    {
        return Err(RemoteError::InvalidRuntime(INVALID_STAGE.into()));
    }
    Ok(())
}
