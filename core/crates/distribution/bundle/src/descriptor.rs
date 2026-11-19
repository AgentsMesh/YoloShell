use crate::{
    ArtifactDescriptor, ArtifactRole, BUNDLE_SCHEMA_VERSION, BundleArtifacts, BundleError,
    RELEASES_DIRECTORY, Target, binary_target, identity, validation,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "BundleFields", into = "BundleFields")]
pub struct BundleDescriptor(BundleFields);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BundleFields {
    pub schema_version: u16,
    pub source_revision: String,
    pub version: String,
    pub target: Target,
    pub build_id: String,
    pub artifacts: BundleArtifacts,
}

impl BundleDescriptor {
    pub fn new(
        source_revision: impl Into<String>,
        version: impl Into<String>,
        target: Target,
        artifacts: BundleArtifacts,
    ) -> Result<Self, BundleError> {
        let mut fields = BundleFields {
            schema_version: BUNDLE_SCHEMA_VERSION,
            source_revision: source_revision.into(),
            version: version.into(),
            target,
            build_id: String::new(),
            artifacts,
        };
        validate_metadata(&fields)?;
        fields.build_id = identity::build_id(&fields);
        Ok(Self(fields))
    }

    pub fn fields(&self) -> &BundleFields {
        &self.0
    }
    pub fn build_id(&self) -> &str {
        &self.0.build_id
    }
    pub fn source_revision(&self) -> &str {
        &self.0.source_revision
    }
    pub fn version(&self) -> &str {
        &self.0.version
    }
    pub fn target(&self) -> Target {
        self.0.target
    }
    pub fn artifact(&self, role: ArtifactRole) -> &ArtifactDescriptor {
        self.0.artifacts.artifact(role)
    }

    pub fn verify_artifact(&self, role: ArtifactRole, bytes: &[u8]) -> Result<(), BundleError> {
        let artifact = self.artifact(role);
        if bytes.len() as u64 != artifact.size_bytes()
            || yolo_crypto::sha256_hex(bytes) != artifact.sha256()
        {
            return Err(BundleError::ArtifactIntegrity);
        }
        if binary_target(bytes) != Some(self.target()) {
            return Err(BundleError::ArtifactTarget);
        }
        Ok(())
    }

    pub fn release_relative_path(&self) -> PathBuf {
        Path::new(RELEASES_DIRECTORY).join(self.build_id())
    }

    /// Pure path derivation. The caller owns installation-root safety and filesystem custody.
    pub fn release_directory(&self, install_root: &Path) -> PathBuf {
        install_root.join(self.release_relative_path())
    }

    pub fn executable_path(&self, install_root: &Path, role: ArtifactRole) -> PathBuf {
        self.release_directory(install_root)
            .join(role.relative_path())
    }
}

impl TryFrom<BundleFields> for BundleDescriptor {
    type Error = BundleError;

    fn try_from(fields: BundleFields) -> Result<Self, Self::Error> {
        validate_metadata(&fields)?;
        if !validation::digest(&fields.build_id) || identity::build_id(&fields) != fields.build_id {
            return Err(BundleError::BuildIdentity);
        }
        Ok(Self(fields))
    }
}

impl From<BundleDescriptor> for BundleFields {
    fn from(value: BundleDescriptor) -> Self {
        value.0
    }
}

fn validate_metadata(fields: &BundleFields) -> Result<(), BundleError> {
    if fields.schema_version != BUNDLE_SCHEMA_VERSION {
        return Err(BundleError::Schema);
    }
    validation::source_revision(&fields.source_revision)?;
    validation::version(&fields.version)
}
