use super::*;

pub(crate) const REDACTED_CREATION_RECORD_SECRET: &str = "[REDACTED]";

impl std::fmt::Debug for CreationSpec {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CreationSpec")
            .field(field::REQUEST_FINGERPRINT, &self.request_fingerprint)
            .field(field::DATA, &REDACTED_CREATION_RECORD_SECRET)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for CreationLaunchIdentity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CreationLaunchIdentity")
            .field(field::RUNTIME_EPOCH, &self.runtime_epoch)
            .field(field::WORKER_ID, &self.worker_id)
            .field(field::AUTH_TOKEN, &REDACTED_CREATION_RECORD_SECRET)
            .finish()
    }
}

impl std::fmt::Debug for CreationBinding {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CreationBinding")
            .field(field::CREATION_TOKEN_DIGEST, &self.creation_token_digest)
            .field(field::RESULT, &self.result)
            .field(field::SPEC, &self.spec)
            .field(field::PHASE, &self.phase)
            .field(field::GENERATION, &self.launch_generation)
            .field(field::LAUNCH, &self.launch)
            .finish()
    }
}

impl std::fmt::Debug for CreationReceipt {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CreationReceipt")
            .field(field::SCHEMA_VERSION, &self.schema_version)
            .field(field::CREATION_TOKEN_DIGEST, &self.creation_token_digest)
            .field(field::RESULT, &self.result)
            .field(field::COMPLETION, &self.completion)
            .finish()
    }
}

impl std::fmt::Debug for CreationLedger {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CreationLedger")
            .field(field::SCHEMA_VERSION, &self.schema_version)
            .field(field::BINDINGS, &self.bindings)
            .field(field::POISONED, &self.poisoned)
            .finish_non_exhaustive()
    }
}
use crate::protocol::field;
