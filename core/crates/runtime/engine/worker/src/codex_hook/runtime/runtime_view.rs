use super::{CodexAuthorityView, CodexHookRuntime};
#[cfg(test)]
use crate::agent::hooks::NormalizedLifecycle;
#[cfg(test)]
use anyhow::Result;

impl CodexHookRuntime {
    #[cfg(test)]
    pub(in crate::worker::codex_hook) fn state(&self) -> Result<Option<NormalizedLifecycle>> {
        let lifecycle = self.lock_lifecycle_anyhow()?;
        if lifecycle.storage_failed() {
            return Ok(None);
        }
        let _proof = lifecycle.prove_snapshot_current()?;
        Ok(lifecycle.lifecycle())
    }

    pub(in crate::worker::codex_hook) fn outbox_path(&self) -> std::path::PathBuf {
        self.outbox.path().to_path_buf()
    }

    pub(in crate::worker::codex_hook) fn snapshot_path(&self) -> std::path::PathBuf {
        self.artifacts.snapshot_path()
    }

    pub(in crate::worker) fn subscribe_authority(
        &self,
    ) -> tokio::sync::watch::Receiver<CodexAuthorityView> {
        self.registration.control().authority_tx.subscribe()
    }
}
