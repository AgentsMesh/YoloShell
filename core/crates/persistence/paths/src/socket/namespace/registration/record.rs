use super::{NamespaceBinding, constants::*};
use crate::paths::identity::DurableObjectIdentity;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Record {
    schema_version: u16,
    home: DurableObjectIdentity,
    binding: NamespaceBinding,
}

impl Record {
    pub fn new(home: DurableObjectIdentity, binding: &NamespaceBinding) -> Result<Self> {
        let record = Self {
            schema_version: REGISTRATION_SCHEMA,
            home,
            binding: binding.clone(),
        };
        record.validate(home, binding)?;
        Ok(record)
    }
    pub fn validate(&self, home: DurableObjectIdentity, binding: &NamespaceBinding) -> Result<()> {
        ensure!(
            self.schema_version == REGISTRATION_SCHEMA
                && self.home == home
                && self.binding == *binding
                && binding.schema_version == super::super::SOCKET_BINDING_SCHEMA_VERSION
                && binding.effective_user_id == unsafe { libc::geteuid() }
                && binding.home_digest.len() == super::super::SOCKET_FULL_HASH_HEX_LENGTH
                && binding
                    .home_digest
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
            INVALID_REGISTRATION
        );
        Ok(())
    }
}
