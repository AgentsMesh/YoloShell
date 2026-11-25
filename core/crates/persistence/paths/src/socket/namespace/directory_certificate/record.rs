use super::{NamespaceBinding, constants::*};
use crate::paths::identity::DurableObjectIdentity;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in super::super) struct Certificate {
    schema_version: u16,
    directory: DurableObjectIdentity,
    pub binding: NamespaceBinding,
    pub registration: DurableObjectIdentity,
}

impl Certificate {
    pub fn new(
        directory: DurableObjectIdentity,
        binding: NamespaceBinding,
        registration: DurableObjectIdentity,
    ) -> Result<Self> {
        let certificate = Self {
            schema_version: CERTIFICATE_SCHEMA_VERSION,
            directory,
            binding,
            registration,
        };
        certificate.validate(directory)?;
        Ok(certificate)
    }

    pub fn validate(&self, directory: DurableObjectIdentity) -> Result<()> {
        let binding = &self.binding;
        // SAFETY: geteuid only reads the current process credentials.
        let owner = unsafe { libc::geteuid() };
        ensure!(
            self.schema_version == CERTIFICATE_SCHEMA_VERSION
                && self.directory == directory
                && directory.kind() == yolo_durable_identity::ObjectKind::Directory
                && self.registration.kind() == yolo_durable_identity::ObjectKind::RegularFile
                && binding.schema_version == super::super::SOCKET_BINDING_SCHEMA_VERSION
                && binding.effective_user_id == owner
                && binding.home_digest.len() == super::super::SOCKET_FULL_HASH_HEX_LENGTH
                && binding
                    .home_digest
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
            INVALID_CERTIFICATE
        );
        Ok(())
    }
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../../../unit_tests/socket_namespace/socket_namespace_directory_certificate_record_tests.rs"]
mod tests;
