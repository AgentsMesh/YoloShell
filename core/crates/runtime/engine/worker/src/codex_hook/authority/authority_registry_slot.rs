use super::super::authority_control::CodexAuthorityControl;
use super::{CodexAuthorityEntry, CodexAuthoritySlot, CodexAuthorityToken, CodexCurrentAuthority};
use anyhow::Result;
use std::sync::{Arc, MutexGuard, Weak};

const CURRENT_LOCK_ERROR: &str = "Codex authority registry current lock is poisoned";
const CURRENT_DROPPED_ERROR: &str = "Codex authority registry control was dropped early";

impl CodexAuthoritySlot {
    pub(super) fn lock_current(&self) -> Result<MutexGuard<'_, Option<CodexAuthorityEntry>>> {
        self.current
            .lock()
            .map_err(|_| anyhow::anyhow!(CURRENT_LOCK_ERROR))
    }

    pub(super) fn observe_current(
        current: &Option<CodexAuthorityEntry>,
    ) -> Result<Option<CodexCurrentAuthority>> {
        current
            .as_ref()
            .map(|entry| {
                Ok(CodexCurrentAuthority {
                    token: entry.token,
                    control: entry
                        .control
                        .upgrade()
                        .ok_or_else(|| anyhow::anyhow!(CURRENT_DROPPED_ERROR))?,
                })
            })
            .transpose()
    }

    pub(super) fn remove_exact(
        &self,
        token: CodexAuthorityToken,
        control: &Arc<CodexAuthorityControl>,
    ) -> Result<()> {
        let mut current = self.lock_current()?;
        let control = Arc::downgrade(control);
        let matches = current
            .as_ref()
            .is_some_and(|entry| entry.token == token && Weak::ptr_eq(&entry.control, &control));
        if matches {
            current.take();
        }
        Ok(())
    }
}
