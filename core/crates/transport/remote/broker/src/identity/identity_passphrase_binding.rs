use super::config_validation::validate_host_id;
use super::credential_binding::{
    CREDENTIAL_BINDING_ERROR, CREDENTIAL_BINDING_MISMATCH, CredentialBindingFile,
    ordered_identity_paths_sha256, validate_digest, validate_nonce,
};
use super::identity_passphrases::wire::EnvelopeBinding;
use super::identity_passphrases::{CREDENTIAL_REVISION_ERROR, IdentityPassphrases};
use anyhow::{Context, Result, ensure};
use std::path::PathBuf;

impl IdentityPassphrases {
    pub(in crate::remote_broker) fn validate_source(
        &self,
        host_id: &str,
        revision: Option<&str>,
        paths: &[PathBuf],
        configured: Option<&CredentialBindingFile>,
    ) -> Result<Option<String>> {
        if let Some(value) = configured {
            value.validate()?;
            ensure!(
                ordered_identity_paths_sha256(paths)? == value.identity_paths_sha256,
                CREDENTIAL_BINDING_MISMATCH
            );
        }
        if !self.provided {
            return Ok(configured.map(|value| value.nonce.clone()));
        }
        let configured = configured.context(CREDENTIAL_BINDING_ERROR)?;
        let binding = self.validate_stable(host_id, revision, &configured.identity_paths_sha256)?;
        ensure!(
            binding.binding_nonce == configured.nonce,
            CREDENTIAL_BINDING_MISMATCH
        );
        Ok(Some(configured.nonce.clone()))
    }

    pub(in crate::remote_broker) fn validate_snapshot(
        &self,
        host_id: &str,
        revision: Option<&str>,
        configured: Option<&CredentialBindingFile>,
        require_nonce: bool,
    ) -> Result<Option<String>> {
        if !self.provided {
            return Ok(configured.map(|value| value.nonce.clone()));
        }
        let configured = configured.context(CREDENTIAL_BINDING_ERROR)?;
        configured.validate()?;
        let binding = self.validate_stable(host_id, revision, &configured.identity_paths_sha256)?;
        if require_nonce {
            ensure!(
                binding.binding_nonce == configured.nonce,
                CREDENTIAL_BINDING_MISMATCH
            );
        }
        Ok(Some(configured.nonce.clone()))
    }

    fn validate_stable<'a>(
        &'a self,
        host_id: &str,
        revision: Option<&str>,
        digest: &str,
    ) -> Result<&'a EnvelopeBinding> {
        let binding = self.binding.as_ref().context(CREDENTIAL_BINDING_ERROR)?;
        ensure!(binding.host_id == host_id, CREDENTIAL_BINDING_MISMATCH);
        ensure!(
            binding.identity_credential_revision.as_deref() == revision,
            CREDENTIAL_BINDING_MISMATCH
        );
        ensure!(
            binding.identity_paths_sha256 == digest,
            CREDENTIAL_BINDING_MISMATCH
        );
        Ok(binding)
    }
}

impl EnvelopeBinding {
    pub(super) fn validate(&self) -> Result<()> {
        validate_nonce(&self.binding_nonce)?;
        validate_host_id(&self.host_id)?;
        validate_digest(&self.identity_paths_sha256)?;
        if let Some(revision) = &self.identity_credential_revision {
            ensure!(
                uuid::Uuid::parse_str(revision).is_ok(),
                CREDENTIAL_REVISION_ERROR
            );
        }
        Ok(())
    }
}
