use super::{CodexAdmissionPublication, CodexAdmissionRecord, CodexAdmissionStorage};
use crate::paths::identity::FileIdentity;
use anyhow::{Context, Result, ensure};

const CODEX_ADMISSION_MAX_BYTES: u64 = crate::paths::MAX_DURABLE_JSON_BYTES;
const ADMISSION_CONFLICT: &str = "Codex invocation admission conflicts with durable authority";
const ADMISSION_MISSING: &str = "Codex invocation admission is missing after publication";

impl CodexAdmissionStorage {
    pub(crate) fn publish(
        &self,
        record: &CodexAdmissionRecord,
    ) -> Result<CodexAdmissionPublication> {
        self.publish_with_commit_fence(record, || Ok(()))
    }

    pub fn publish_with_commit_fence<F>(
        &self,
        record: &CodexAdmissionRecord,
        mut commit_fence: F,
    ) -> Result<CodexAdmissionPublication>
    where
        F: FnMut() -> Result<()>,
    {
        record.validate_for(&self.owner)?;
        commit_fence()?;
        let encoded = serde_json::to_vec_pretty(record)?;
        ensure!(
            u64::try_from(encoded.len())? <= CODEX_ADMISSION_MAX_BYTES,
            ADMISSION_CONFLICT
        );
        let entry = self.entry()?;
        match entry.create_bytes_exclusive_with_fence(
            &encoded,
            CODEX_ADMISSION_MAX_BYTES,
            &mut commit_fence,
        ) {
            Ok(created) => self.finish_created(record, created),
            Err(cause) => {
                commit_fence()?;
                self.finish_replay(record, cause)
            }
        }
    }

    fn finish_created(
        &self,
        expected: &CodexAdmissionRecord,
        created: FileIdentity,
    ) -> Result<CodexAdmissionPublication> {
        let artifact = self.read()?.context(ADMISSION_MISSING)?;
        ensure!(
            artifact.record == *expected && artifact.file_identity == created,
            ADMISSION_CONFLICT
        );
        Ok(CodexAdmissionPublication::new(artifact, false))
    }

    fn finish_replay(
        &self,
        expected: &CodexAdmissionRecord,
        cause: anyhow::Error,
    ) -> Result<CodexAdmissionPublication> {
        match self.read() {
            Ok(Some(artifact)) if artifact.record == *expected => {
                Ok(CodexAdmissionPublication::new(artifact, true))
            }
            Ok(Some(_)) => Err(anyhow::anyhow!(ADMISSION_CONFLICT)),
            Ok(None) => Err(cause.context(ADMISSION_CONFLICT)),
            Err(error) => Err(error.context(ADMISSION_CONFLICT)),
        }
    }
}
