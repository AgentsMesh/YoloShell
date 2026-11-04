use super::CodexAdmissionStorage;
use anyhow::{Context, Result};

const ADMISSION_LOCK_ERROR: &str = "lock Codex invocation admission publication";

impl CodexAdmissionStorage {
    pub fn lock_publication(&self) -> Result<crate::paths::private_file::PrivateDirectoryLock> {
        self.entry()?
            .lock_parent_exclusive_wait(
                super::constants::LOCK_TIMEOUT,
                super::constants::LOCK_RETRY,
            )
            .context(ADMISSION_LOCK_ERROR)
    }
}
