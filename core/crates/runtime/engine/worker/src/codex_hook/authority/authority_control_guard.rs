use super::CodexAuthorityRegistration;
use crate::worker::codex_hook::authority::LifecycleAuthority;
use anyhow::{Context, Result, ensure};
use std::sync::{Arc, atomic::Ordering};

const UNAVAILABLE: &str = "Codex control registration is busy, replaced, or unavailable";

impl CodexAuthorityRegistration {
    pub(in crate::worker::codex_hook) fn with_control_authority<T>(
        &self,
        operation: impl FnOnce(&LifecycleAuthority) -> Result<T>,
    ) -> Result<T> {
        let _takeover = self
            .slot
            .takeover
            .try_lock()
            .map_err(|_| anyhow::anyhow!(UNAVAILABLE))?;
        let current = self
            .slot
            .current
            .try_lock()
            .map_err(|_| anyhow::anyhow!(UNAVAILABLE))?;
        let current = current.as_ref().context(UNAVAILABLE)?;
        ensure!(
            self.installed.load(Ordering::Acquire)
                && current.token == self.token
                && current
                    .control
                    .upgrade()
                    .is_some_and(|control| Arc::ptr_eq(&control, &self.control)),
            UNAVAILABLE
        );
        let lifecycle = self
            .control
            .lifecycle
            .try_lock()
            .map_err(|_| anyhow::anyhow!(UNAVAILABLE))?;
        operation(&lifecycle)
    }
}
