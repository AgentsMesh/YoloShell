use super::*;

pub(crate) const CREATION_RECEIPT_FINGERPRINT_INVALID: &str =
    "creation receipt request fingerprint is invalid";
pub(crate) const CREATION_RECEIPT_BINDING_FINGERPRINT_CONFLICT: &str =
    "creation receipt request differs from its active binding";

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreationReceipt {
    pub(crate) schema_version: u16,
    pub(crate) creation_token_digest: String,
    pub(crate) request_fingerprint: String,
    #[serde(flatten)]
    pub(crate) result: CreateTerminalResult,
    pub(crate) completion: CreationCompletion,
}

impl CreationReceipt {
    pub(crate) fn validate_binding_fingerprint(&self, binding: &CreationBinding) -> Result<()> {
        anyhow::ensure!(
            self.request_fingerprint == binding.spec.request_fingerprint,
            CREATION_RECEIPT_BINDING_FINGERPRINT_CONFLICT
        );
        Ok(())
    }

    pub(crate) fn from_binding(binding: &CreationBinding, completion: CreationCompletion) -> Self {
        Self {
            schema_version: CREATION_RECEIPT_SCHEMA_VERSION,
            creation_token_digest: binding.creation_token_digest.clone(),
            request_fingerprint: binding.spec.request_fingerprint.clone(),
            result: binding.result,
            completion,
        }
    }

    pub(crate) fn validate(&self) -> Result<()> {
        anyhow::ensure!(
            self.schema_version == CREATION_RECEIPT_SCHEMA_VERSION,
            "unsupported creation receipt schema {}",
            self.schema_version
        );
        anyhow::ensure!(
            valid_creation_digest(&self.request_fingerprint),
            CREATION_RECEIPT_FINGERPRINT_INVALID
        );
        anyhow::ensure!(
            valid_creation_digest(&self.creation_token_digest),
            "invalid creation receipt token digest"
        );
        anyhow::ensure!(
            !self.result.terminal_id.is_nil(),
            "nil receipt session identity"
        );
        anyhow::ensure!(
            !self.result.creation_capability.is_nil(),
            "nil receipt creation capability"
        );
        Ok(())
    }
}
