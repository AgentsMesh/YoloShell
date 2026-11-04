use super::{CodexRetirementWriterFence, run_after_acquire_hook};
use crate::codex_hooks::retirement_constants::STORAGE;
use crate::codex_hooks::retirement_storage::CodexRetirementStorage;
use crate::codex_hooks::{CodexAdmissionStorage, CodexHookIntake};
use anyhow::{Context, Result};

impl CodexHookIntake {
    pub(crate) fn acquire_writer_fence(&self) -> Result<CodexRetirementWriterFence> {
        let storage = CodexRetirementStorage::for_intake(
            self.invocation(),
            self.directory().path(),
            self.owner(),
        )?;
        CodexRetirementWriterFence::acquire(storage)
    }
}

impl CodexAdmissionStorage {
    pub fn acquire_retirement_writer_fence(&self) -> Result<CodexRetirementWriterFence> {
        CodexRetirementWriterFence::acquire(CodexRetirementStorage::for_admission(self)?)
    }
}

impl CodexRetirementWriterFence {
    fn acquire(storage: CodexRetirementStorage) -> Result<Self> {
        let directory = storage.directory().try_clone()?;
        let lock = crate::paths::private_file_lock::PrivateFileLock::acquire_in_wait(
            directory,
            storage.writer_lock_path(),
            crate::codex_hooks::constants::LOCK_RECORD_MAX_BYTES,
            crate::codex_hooks::constants::RETIREMENT_WRITER_TIMEOUT,
            crate::codex_hooks::constants::LOCK_RETRY,
        )
        .context(STORAGE)?;
        let fence = Self { storage, lock };
        fence.verify_lock()?;
        run_after_acquire_hook();
        fence.verify_lock()?;
        Ok(fence)
    }
}
