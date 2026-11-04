use super::super::authority::LifecycleAuthority;
use super::super::constants::CODEX_STORAGE_LATCHED_ERROR;
use super::super::error::CodexHookRejection;
use super::CodexHookRuntime;
use anyhow::Result;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{MutexGuard, PoisonError};

impl CodexHookRuntime {
    pub(super) fn lock_lifecycle(
        &self,
    ) -> Result<MutexGuard<'_, LifecycleAuthority>, CodexHookRejection> {
        match self.registration.control().lifecycle.lock() {
            Ok(lifecycle) => Ok(lifecycle),
            Err(poisoned) => {
                self.latch_poisoned(poisoned);
                Err(CodexHookRejection::Storage(anyhow::anyhow!(
                    CODEX_STORAGE_LATCHED_ERROR
                )))
            }
        }
    }

    pub(in crate::worker::codex_hook) fn lock_lifecycle_anyhow(
        &self,
    ) -> Result<MutexGuard<'_, LifecycleAuthority>> {
        match self.registration.control().lifecycle.lock() {
            Ok(lifecycle) => Ok(lifecycle),
            Err(poisoned) => {
                self.latch_poisoned(poisoned);
                Err(anyhow::anyhow!(CODEX_STORAGE_LATCHED_ERROR))
            }
        }
    }

    pub(in crate::worker::codex_hook) fn contain_authority_panic(&self) {
        let changed = match self.registration.control().lifecycle.lock() {
            Ok(mut lifecycle) => lifecycle.latch_storage_failure(),
            Err(poisoned) => {
                let changed = poisoned.into_inner().latch_storage_failure();
                self.registration.control().lifecycle.clear_poison();
                if changed {
                    self.publish_storage_unavailable();
                }
                return;
            }
        };
        if changed {
            self.publish_storage_unavailable();
        }
    }

    pub(super) fn contain_authority_unwind<T, E>(
        &self,
        operation: impl FnOnce() -> std::result::Result<T, E>,
        failure: impl FnOnce() -> E,
    ) -> std::result::Result<T, E> {
        match catch_unwind(AssertUnwindSafe(operation)) {
            Ok(result) => result,
            Err(payload) => {
                self.contain_authority_panic();
                std::mem::forget(payload);
                Err(failure())
            }
        }
    }

    fn latch_poisoned(&self, poisoned: PoisonError<MutexGuard<'_, LifecycleAuthority>>) {
        let changed = poisoned.into_inner().latch_storage_failure();
        self.registration.control().lifecycle.clear_poison();
        if changed {
            self.publish_storage_unavailable();
        }
    }
}
