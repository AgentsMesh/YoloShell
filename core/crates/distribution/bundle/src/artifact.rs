use crate::{ArtifactRole, BundleError, MAX_ARTIFACT_BYTES, validation};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "ArtifactFields", into = "ArtifactFields")]
pub struct ArtifactDescriptor(ArtifactFields);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactFields {
    pub sha256: String,
    pub size_bytes: u64,
}

impl ArtifactDescriptor {
    pub fn new(sha256: impl Into<String>, size_bytes: u64) -> Result<Self, BundleError> {
        ArtifactFields {
            sha256: sha256.into(),
            size_bytes,
        }
        .try_into()
    }

    pub fn fields(&self) -> &ArtifactFields {
        &self.0
    }
    pub fn sha256(&self) -> &str {
        &self.0.sha256
    }
    pub fn size_bytes(&self) -> u64 {
        self.0.size_bytes
    }
}

impl TryFrom<ArtifactFields> for ArtifactDescriptor {
    type Error = BundleError;

    fn try_from(fields: ArtifactFields) -> Result<Self, Self::Error> {
        if !validation::digest(&fields.sha256) {
            return Err(BundleError::Digest);
        }
        if fields.size_bytes == 0 || fields.size_bytes > MAX_ARTIFACT_BYTES {
            return Err(BundleError::Size);
        }
        Ok(Self(fields))
    }
}

impl From<ArtifactDescriptor> for ArtifactFields {
    fn from(value: ArtifactDescriptor) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BundleArtifacts {
    pub runtime: ArtifactDescriptor,
}

impl BundleArtifacts {
    pub fn artifact(&self, role: ArtifactRole) -> &ArtifactDescriptor {
        match role {
            ArtifactRole::Runtime => &self.runtime,
        }
    }
}
