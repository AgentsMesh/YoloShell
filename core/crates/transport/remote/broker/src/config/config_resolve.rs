use super::config::{BindingMode, BrokerConfig, ConfigFile};
use super::config_artifact::load_artifact;
use super::config_validation::{
    resolve_agent_socket, validate_host_id, validate_profile_authority_sha256,
};
use super::constants::*;
use super::identity_passphrases::CREDENTIAL_REVISION_ERROR;
use super::revision::config_revision;
use super::{IdentityPassphrases, security};
use crate::remote::{
    CommandPolicy, FileReadPolicy, IdentityFile, KnownHosts, RemoteEndpoint, RemoteRuntimeSpec,
    RuntimeArtifact,
};
use anyhow::{Context, Result, ensure};
use std::sync::Arc;

impl ConfigFile {
    pub(super) fn validate_source_binding(&self, passphrases: &IdentityPassphrases) -> Result<()> {
        self.validate_common(passphrases)?;
        passphrases.validate_source(
            &self.host_id,
            self.identity_credential_revision.as_deref(),
            &self.identities,
            self.credential_binding.as_ref(),
        )?;
        Ok(())
    }

    pub(super) fn resolve(
        self,
        passphrases: &IdentityPassphrases,
        mode: BindingMode,
    ) -> Result<(BrokerConfig, Option<String>)> {
        let revision_material = super::config_snapshot::revision_material(&self)?;
        self.validate_common(passphrases)?;
        let nonce = match mode {
            BindingMode::Source => passphrases.validate_source(
                &self.host_id,
                self.identity_credential_revision.as_deref(),
                &self.identities,
                self.credential_binding.as_ref(),
            )?,
            BindingMode::SnapshotExact | BindingMode::SnapshotRebound => passphrases
                .validate_snapshot(
                    &self.host_id,
                    self.identity_credential_revision.as_deref(),
                    self.credential_binding.as_ref(),
                    mode == BindingMode::SnapshotExact,
                )?,
        };
        Ok((
            self.resolve_material(passphrases, revision_material)?,
            nonce,
        ))
    }

    fn validate_common(&self, passphrases: &IdentityPassphrases) -> Result<()> {
        ensure!(
            !self.hub_transport || self.artifact.is_none(),
            yolo_broker_hub_protocol::BrokerHubError::Artifact
        );
        ensure!(
            self.version == BROKER_CONFIG_VERSION,
            "unsupported broker config version"
        );
        validate_host_id(&self.host_id)?;
        validate_profile_authority_sha256(&self.profile_authority_sha256)?;
        ensure!(
            self.identities.len() <= MAX_IDENTITY_FILES,
            "{SSH_IDENTITY_COUNT_LIMIT_MESSAGE}"
        );
        passphrases.validate_count(self.identities.len())?;
        if let Some(revision) = &self.identity_credential_revision {
            ensure!(
                uuid::Uuid::parse_str(revision).is_ok(),
                "{CREDENTIAL_REVISION_ERROR}"
            );
        }
        if let Some(binding) = &self.credential_binding {
            binding.validate()?;
        }
        Ok(())
    }

    fn resolve_material(
        self,
        passphrases: &IdentityPassphrases,
        revision_material: Vec<u8>,
    ) -> Result<BrokerConfig> {
        let known_hosts_bytes = security::read_trusted_file(&self.known_hosts, KNOWN_HOSTS_LABEL)?;
        let known_hosts_text =
            std::str::from_utf8(&known_hosts_bytes).context("known_hosts is not UTF-8")?;
        let known_hosts = KnownHosts::parse(known_hosts_text)?;
        let identity_bytes = self
            .identities
            .iter()
            .map(|path| security::read_identity(path))
            .collect::<Result<Vec<_>>>()?;
        ensure!(
            self.use_agent || self.agent_socket.is_none(),
            "agent_socket requires use_agent"
        );
        let agent_socket = resolve_agent_socket(self.use_agent, self.agent_socket)?;
        if let Some(path) = &agent_socket {
            security::validate_agent_socket(path)?;
        }
        let endpoint = match self.endpoint.port {
            Some(port) => RemoteEndpoint::new(self.endpoint.user, self.endpoint.host, port)?,
            None => RemoteEndpoint::standard(self.endpoint.user, self.endpoint.host)?,
        };
        let runtime = RemoteRuntimeSpec::new(self.runtime.executable, self.runtime.home)?;
        let command_policy = CommandPolicy::new(self.policy.allowed_programs)?;
        let file_policy = FileReadPolicy::standard(self.policy.read_roots)?;
        let artifact = self.artifact.map(load_artifact).transpose()?;
        let revision = config_revision(
            &revision_material,
            &known_hosts_bytes,
            &identity_bytes,
            agent_socket.as_deref(),
            artifact.as_ref().map(RuntimeArtifact::bytes),
        );
        let identities = self
            .identities
            .into_iter()
            .zip(identity_bytes)
            .enumerate()
            .map(|(index, (path, material))| {
                IdentityFile::from_material(path, Arc::from(material), passphrases.get(index))
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(BrokerConfig {
            hub_transport: self.hub_transport,
            host_id: self.host_id,
            profile_authority_sha256: self.profile_authority_sha256,
            endpoint,
            known_hosts,
            identities,
            agent_socket,
            runtime,
            command_policy,
            file_policy,
            artifact,
            revision,
        })
    }
}
