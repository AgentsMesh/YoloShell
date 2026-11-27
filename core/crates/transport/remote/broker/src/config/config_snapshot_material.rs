use super::super::config::{ConfigFile, resolve_agent_socket};
use super::super::constants::*;
use super::super::revision::config_revision;
use super::super::security;
use super::IdentityPassphrases;
use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};

pub(super) struct SnapshotMaterial {
    pub(super) raw: ConfigFile,
    pub(super) known_hosts: Vec<u8>,
    pub(super) identities: Vec<Vec<u8>>,
    pub(super) artifact: Option<Vec<u8>>,
    pub(super) revision: String,
}

pub(super) fn read(source: &Path, passphrases: &IdentityPassphrases) -> Result<SnapshotMaterial> {
    let config_bytes = security::read_private_file(source, CONFIG_LABEL)?;
    let raw: ConfigFile =
        serde_json::from_slice(&config_bytes).context("decode broker config snapshot")?;
    raw.validate_source_binding(passphrases)?;
    ensure!(
        raw.identities.len() <= MAX_IDENTITY_FILES,
        "{SSH_IDENTITY_COUNT_LIMIT_MESSAGE}"
    );
    let known_hosts = security::read_trusted_file(&raw.known_hosts, KNOWN_HOSTS_LABEL)?;
    let identities = raw
        .identities
        .iter()
        .map(|path| security::read_identity(path))
        .collect::<Result<Vec<_>>>()?;
    let artifact = raw
        .artifact
        .as_ref()
        .map(|value| security::read_trusted_file(&value.path, ARTIFACT_LABEL))
        .transpose()?;
    let agent_socket = resolve_agent_socket(raw.use_agent, raw.agent_socket.clone())?;
    let material = revision_material(&raw)?;
    let revision = config_revision(
        &material,
        &known_hosts,
        &identities,
        agent_socket.as_deref(),
        artifact.as_deref(),
    );
    Ok(SnapshotMaterial {
        raw,
        known_hosts,
        identities,
        artifact,
        revision,
    })
}

pub(in crate::remote_broker) fn revision_material(raw: &ConfigFile) -> Result<Vec<u8>> {
    let mut normalized = raw.clone();
    normalized.known_hosts = PathBuf::new();
    normalized.identities.fill(PathBuf::new());
    if let Some(binding) = &mut normalized.credential_binding {
        binding.nonce.clear();
    }
    if let Some(artifact) = &mut normalized.artifact {
        artifact.path = PathBuf::new();
    }
    serde_json::to_vec(&normalized).context("encode broker revision material")
}

pub(super) fn rewrite_paths(raw: &mut ConfigFile, destination: &Path) {
    raw.known_hosts = destination.join(GENERATION_KNOWN_HOSTS_FILE);
    for (index, path) in raw.identities.iter_mut().enumerate() {
        *path = destination.join(identity_name(index));
    }
    if let Some(artifact) = &mut raw.artifact {
        artifact.path = destination.join(GENERATION_ARTIFACT_FILE);
    }
}

pub(super) fn identity_name(index: usize) -> String {
    format!("{GENERATION_IDENTITY_PREFIX}{index}")
}
