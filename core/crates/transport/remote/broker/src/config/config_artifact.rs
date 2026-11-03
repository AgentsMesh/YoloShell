use super::constants::ARTIFACT_LABEL;
use super::security;
use crate::remote::{RemoteArchitecture, RuntimeArtifact};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ArtifactFile {
    pub(super) path: PathBuf,
    version: String,
    architecture: ArtifactArchitecture,
    sha256: String,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum ArtifactArchitecture {
    Aarch64,
    X86_64,
}

pub(super) fn load_artifact(value: ArtifactFile) -> Result<RuntimeArtifact> {
    let bytes = security::read_trusted_file(&value.path, ARTIFACT_LABEL)?;
    let architecture = match value.architecture {
        ArtifactArchitecture::Aarch64 => RemoteArchitecture::Aarch64,
        ArtifactArchitecture::X86_64 => RemoteArchitecture::X86_64,
    };
    Ok(RuntimeArtifact::new(
        value.version,
        architecture,
        bytes,
        value.sha256,
    )?)
}
