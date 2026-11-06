use crate::paths::identity::FileIdentity;
use crate::pty_custody::CustodyGuardianBinding;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustodyGuardianPreparation {
    binding: CustodyGuardianBinding,
    runtime_file_identity: FileIdentity,
    predecessor_file_identity: FileIdentity,
    custody_file_identity: FileIdentity,
}

impl CustodyGuardianPreparation {
    pub fn new(
        binding: CustodyGuardianBinding,
        runtime_file_identity: FileIdentity,
        predecessor_file_identity: FileIdentity,
        custody_file_identity: FileIdentity,
    ) -> Self {
        Self {
            binding,
            runtime_file_identity,
            predecessor_file_identity,
            custody_file_identity,
        }
    }

    pub fn binding(self) -> CustodyGuardianBinding {
        self.binding
    }

    pub fn custody_file_identity(self) -> FileIdentity {
        self.custody_file_identity
    }

    pub fn predecessor_file_identity(self) -> FileIdentity {
        self.predecessor_file_identity
    }

    pub fn runtime_file_identity(self) -> FileIdentity {
        self.runtime_file_identity
    }

    pub fn with_runtime_file_identity(self, identity: FileIdentity) -> Self {
        Self {
            runtime_file_identity: identity,
            ..self
        }
    }

    pub(crate) fn with_predecessor_file_identity(self, identity: FileIdentity) -> Self {
        Self {
            predecessor_file_identity: identity,
            ..self
        }
    }

    pub fn with_custody_file_identity(self, identity: FileIdentity) -> Self {
        Self {
            custody_file_identity: identity,
            ..self
        }
    }
}
