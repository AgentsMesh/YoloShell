use super::authority::LifecycleAuthority;
use super::publication::CodexAuthorityView;
use anyhow::{Result, bail};
use std::sync::MutexGuard;
use std::sync::{Arc, Mutex};

pub(super) struct CodexAuthorityControl {
    pub(super) lifecycle: Mutex<LifecycleAuthority>,
    pub(super) authority_tx: tokio::sync::watch::Sender<CodexAuthorityView>,
}

impl CodexAuthorityControl {
    pub(super) fn new(lifecycle: LifecycleAuthority) -> Arc<Self> {
        let (authority_tx, _) = tokio::sync::watch::channel(CodexAuthorityView::StorageUnavailable);
        Arc::new(Self {
            lifecycle: Mutex::new(lifecycle),
            authority_tx,
        })
    }

    pub(super) fn lock_for_takeover(&self) -> Result<MutexGuard<'_, LifecycleAuthority>> {
        match self.lifecycle.lock() {
            Ok(lifecycle) => Ok(lifecycle),
            Err(poisoned) => {
                let changed = poisoned.into_inner().latch_storage_failure();
                self.lifecycle.clear_poison();
                if changed {
                    self.publish_terminal_storage_unavailable();
                }
                bail!(super::constants::CODEX_STORAGE_LATCHED_ERROR)
            }
        }
    }
}
