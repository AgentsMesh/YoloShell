use super::authority_control::CodexAuthorityControl;
use crate::codex_hooks::CodexSnapshotClaimToken;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use uuid::Uuid;

#[path = "authority_registration.rs"]
mod registration;
#[path = "authority_registry_slot.rs"]
mod slot;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../../test_support/unit/codex_hook/authority/authority_registry_test_support.rs"]
mod test_support;
pub(super) use registration::{CodexAuthorityPreparedRegistration, CodexAuthorityRegistration};
#[cfg(test)]
pub(in crate::worker::codex_hook) use test_support::{
    CodexRegistryPoisonTarget, RetainedDeadCodexAuthority,
};

const SLOT_RESERVE_ERROR: &str = "reserve Codex authority registry slot";
const REGISTRY_LOCK_ERROR: &str = "Codex authority registry lock is poisoned";
const TAKEOVER_LOCK_ERROR: &str = "Codex authority takeover gate is poisoned";
const CURRENT_CONTROL_ERROR: &str = "Codex authority registry current control is unavailable";

#[derive(Default)]
pub(in crate::worker::codex_hook) struct CodexAuthorityRegistry {
    slots: Mutex<HashMap<CodexAuthorityKey, Weak<CodexAuthoritySlot>>>,
    route_publication_revision: Arc<std::sync::atomic::AtomicU64>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct CodexAuthorityKey {
    terminal_id: Uuid,
    runtime_epoch: Uuid,
    invocation_id: Uuid,
}

struct CodexAuthoritySlot {
    takeover: Mutex<()>,
    current: Mutex<Option<CodexAuthorityEntry>>,
}

struct CodexAuthorityEntry {
    token: CodexAuthorityToken,
    control: Weak<CodexAuthorityControl>,
}

#[derive(Clone)]
pub(super) struct CodexCurrentAuthority {
    token: CodexAuthorityToken,
    control: Arc<CodexAuthorityControl>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CodexAuthorityToken {
    claim: CodexSnapshotClaimToken,
}

pub(super) struct CodexAuthorityTakeover<'a> {
    slot: &'a Arc<CodexAuthoritySlot>,
    current: MutexGuard<'a, Option<CodexAuthorityEntry>>,
    observed: Option<CodexCurrentAuthority>,
}

pub(super) struct CodexSuccessionProof {
    _private: (),
}

impl CodexAuthorityRegistry {
    pub(super) fn route_publication_clock(&self) -> Arc<std::sync::atomic::AtomicU64> {
        Arc::clone(&self.route_publication_revision)
    }

    pub(super) fn observe_route_publication_floor(&self, revision: u64) {
        self.route_publication_revision
            .fetch_max(revision, std::sync::atomic::Ordering::AcqRel);
    }

    pub(super) fn with_takeover<T>(
        &self,
        key: CodexAuthorityKey,
        operation: impl FnOnce(CodexAuthorityTakeover<'_>) -> Result<T>,
    ) -> Result<T> {
        let slot = self.slot(key)?;
        let _takeover = slot
            .takeover
            .lock()
            .map_err(|_| anyhow::anyhow!(TAKEOVER_LOCK_ERROR))?;
        let current = slot.lock_current()?;
        let observed =
            CodexAuthoritySlot::observe_current(&current).context(CURRENT_CONTROL_ERROR)?;
        operation(CodexAuthorityTakeover {
            slot: &slot,
            current,
            observed,
        })
    }

    fn slot(&self, key: CodexAuthorityKey) -> Result<Arc<CodexAuthoritySlot>> {
        let mut slots = self
            .slots
            .lock()
            .map_err(|_| anyhow::anyhow!(REGISTRY_LOCK_ERROR))?;
        slots.retain(|_, slot| slot.strong_count() != 0);
        if let Some(slot) = slots.get(&key).and_then(Weak::upgrade) {
            return Ok(slot);
        }
        slots.try_reserve(1).context(SLOT_RESERVE_ERROR)?;
        let slot = Arc::new(CodexAuthoritySlot::default());
        slots.insert(key, Arc::downgrade(&slot));
        Ok(slot)
    }
}

impl CodexAuthorityKey {
    pub(super) fn new(terminal_id: Uuid, runtime_epoch: Uuid, invocation_id: Uuid) -> Self {
        Self {
            terminal_id,
            runtime_epoch,
            invocation_id,
        }
    }
}

impl Default for CodexAuthoritySlot {
    fn default() -> Self {
        Self {
            takeover: Mutex::new(()),
            current: Mutex::new(None),
        }
    }
}

impl CodexCurrentAuthority {
    pub(super) fn control(&self) -> &Arc<CodexAuthorityControl> {
        &self.control
    }
}

impl CodexAuthorityTakeover<'_> {
    pub(super) fn current(&self) -> Option<&CodexCurrentAuthority> {
        self.observed.as_ref()
    }

    pub(super) fn prepare_registration(
        &self,
        claim: CodexSnapshotClaimToken,
        control: Arc<CodexAuthorityControl>,
    ) -> CodexAuthorityPreparedRegistration {
        CodexAuthorityPreparedRegistration::new(
            Arc::clone(self.slot),
            CodexAuthorityToken { claim },
            control,
        )
    }

    pub(super) fn install(
        mut self,
        registration: &CodexAuthorityRegistration,
    ) -> Option<CodexSuccessionProof> {
        let succession = self
            .observed
            .take()
            .map(|entry| entry.token)
            .map(|predecessor| registration.succession_from(predecessor));
        *self.current = Some(CodexAuthorityEntry {
            token: registration.token(),
            control: Arc::downgrade(registration.control_arc()),
        });
        registration.mark_installed();
        succession
    }
}
