use super::IdentityPassphrases;
use super::config_artifact::ArtifactFile;
#[cfg(test)]
pub(super) use super::config_validation::validate_host_id;
pub(super) use super::config_validation::{default_use_agent, resolve_agent_socket};
use super::constants::*;
use super::credential_binding::CredentialBindingFile;
use super::security;
use crate::remote::{
    CommandPolicy, FileReadPolicy, IdentityFile, KnownHosts, RemoteEndpoint, RemoteRuntimeSpec,
    RuntimeArtifact,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct BrokerConfig {
    pub(super) hub_transport: bool,
    pub(super) host_id: String,
    pub(super) profile_authority_sha256: String,
    pub(super) endpoint: RemoteEndpoint,
    pub(super) known_hosts: KnownHosts,
    pub(super) identities: Vec<IdentityFile>,
    pub(super) agent_socket: Option<PathBuf>,
    pub(super) runtime: RemoteRuntimeSpec,
    pub(super) command_policy: CommandPolicy,
    pub(super) file_policy: FileReadPolicy,
    pub(super) artifact: Option<RuntimeArtifact>,
    pub(super) revision: String,
}

impl BrokerConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        Self::load_with_passphrases(path, &IdentityPassphrases::none())
    }

    pub fn load_with_passphrases(
        path: impl AsRef<Path>,
        passphrases: &IdentityPassphrases,
    ) -> Result<Self> {
        Self::load_mode(path.as_ref(), passphrases, BindingMode::Source).map(|value| value.0)
    }

    pub fn load_expected(path: impl AsRef<Path>, expected_revision: &str) -> Result<Self> {
        Self::load_expected_with_passphrases(path, expected_revision, &IdentityPassphrases::none())
    }

    pub fn load_expected_with_passphrases(
        path: impl AsRef<Path>,
        expected_revision: &str,
        passphrases: &IdentityPassphrases,
    ) -> Result<Self> {
        let config = Self::load_mode(path.as_ref(), passphrases, BindingMode::SnapshotExact)?.0;
        ensure!(
            config.revision == expected_revision,
            "{BROKER_CONFIG_REVISION_MISMATCH_MESSAGE}"
        );
        Ok(config)
    }

    pub(super) fn load_snapshot_rebound(
        path: &Path,
        passphrases: &IdentityPassphrases,
    ) -> Result<(Self, Option<String>)> {
        Self::load_mode(path, passphrases, BindingMode::SnapshotRebound)
    }

    fn load_mode(
        path: &Path,
        passphrases: &IdentityPassphrases,
        mode: BindingMode,
    ) -> Result<(Self, Option<String>)> {
        let bytes = security::read_private_file(path, CONFIG_LABEL)?;
        let raw: ConfigFile = serde_json::from_slice(&bytes).context("decode broker config")?;
        raw.resolve(passphrases, mode)
    }

    pub fn host_id(&self) -> &str {
        &self.host_id
    }
    pub fn hub_transport(&self) -> bool {
        self.hub_transport
    }
    pub fn profile_authority_sha256(&self) -> &str {
        &self.profile_authority_sha256
    }
    pub fn runtime(&self) -> &RemoteRuntimeSpec {
        &self.runtime
    }
    pub fn revision(&self) -> &str {
        &self.revision
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ConfigFile {
    pub(super) hub_transport: bool,
    pub(super) version: u16,
    pub(super) host_id: String,
    pub(super) profile_authority_sha256: String,
    pub(super) endpoint: EndpointFile,
    pub(super) known_hosts: PathBuf,
    #[serde(default)]
    pub(super) identities: Vec<PathBuf>,
    #[serde(default)]
    pub(super) identity_credential_revision: Option<String>,
    #[serde(default)]
    pub(super) credential_binding: Option<CredentialBindingFile>,
    #[serde(default = "default_use_agent")]
    pub(super) use_agent: bool,
    pub(super) agent_socket: Option<PathBuf>,
    pub(super) runtime: RuntimeFile,
    pub(super) policy: PolicyFile,
    pub(super) artifact: Option<ArtifactFile>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EndpointFile {
    pub(super) user: String,
    pub(super) host: String,
    pub(super) port: Option<u16>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RuntimeFile {
    pub(super) executable: PathBuf,
    pub(super) home: PathBuf,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PolicyFile {
    pub(super) allowed_programs: Vec<String>,
    pub(super) read_roots: Vec<PathBuf>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum BindingMode {
    Source,
    SnapshotExact,
    SnapshotRebound,
}
