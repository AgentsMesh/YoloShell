use super::{CodexAuthorityKey, CodexAuthorityRegistry, CodexAuthoritySlot};
use anyhow::{Context, Result};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex, Weak};
use uuid::Uuid;

const CURRENT_REQUIRED: &str = "Codex authority registry test requires a current entry";
const POISON_PANIC: &str = "injected Codex authority registry poison";

pub(in crate::worker::codex_hook) struct RetainedDeadCodexAuthority {
    _slot: Arc<CodexAuthoritySlot>,
}

#[derive(Clone, Copy)]
pub(in crate::worker::codex_hook) enum CodexRegistryPoisonTarget {
    Registry,
    Takeover,
    Current,
}

impl CodexAuthorityRegistry {
    pub(in crate::worker::codex_hook) fn retain_dead_current_for_test(
        &self,
        terminal_id: Uuid,
        runtime_epoch: Uuid,
        invocation_id: Uuid,
    ) -> Result<RetainedDeadCodexAuthority> {
        let slot = self.slot(CodexAuthorityKey::new(
            terminal_id,
            runtime_epoch,
            invocation_id,
        ))?;
        let takeover = slot
            .takeover
            .lock()
            .map_err(|_| anyhow::anyhow!(super::TAKEOVER_LOCK_ERROR))?;
        let mut current = slot.lock_current()?;
        let entry = current.as_mut().context(CURRENT_REQUIRED)?;
        entry.control = Weak::new();
        drop(current);
        drop(takeover);
        Ok(RetainedDeadCodexAuthority { _slot: slot })
    }

    pub(in crate::worker::codex_hook) fn poison_for_test(
        &self,
        target: CodexRegistryPoisonTarget,
        terminal_id: Uuid,
        runtime_epoch: Uuid,
        invocation_id: Uuid,
    ) {
        if matches!(target, CodexRegistryPoisonTarget::Registry) {
            poison(&self.slots);
            return;
        }
        let slot = self
            .slot(CodexAuthorityKey::new(
                terminal_id,
                runtime_epoch,
                invocation_id,
            ))
            .unwrap();
        match target {
            CodexRegistryPoisonTarget::Registry => unreachable!(),
            CodexRegistryPoisonTarget::Takeover => poison(&slot.takeover),
            CodexRegistryPoisonTarget::Current => poison(&slot.current),
        }
    }
}

fn poison<T>(mutex: &Mutex<T>) {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let _guard = mutex.lock().unwrap();
        panic!("{POISON_PANIC}");
    }));
    assert!(result.is_err());
}
