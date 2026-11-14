use super::super::authority_control::CodexAuthorityControl;
use super::{CodexAuthoritySlot, CodexAuthorityToken, CodexSuccessionProof};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[path = "authority_control_guard.rs"]
mod control_guard;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../../test_support/unit/codex_hook/control/control_registration.rs"]
mod control_test_support;

pub(in crate::worker::codex_hook) struct CodexAuthorityPreparedRegistration {
    slot: Arc<CodexAuthoritySlot>,
    token: CodexAuthorityToken,
    control: Arc<CodexAuthorityControl>,
}

pub(in crate::worker::codex_hook) struct CodexAuthorityRegistration {
    slot: Arc<CodexAuthoritySlot>,
    token: CodexAuthorityToken,
    control: Arc<CodexAuthorityControl>,
    installed: AtomicBool,
}

impl CodexAuthorityPreparedRegistration {
    pub(super) fn new(
        slot: Arc<CodexAuthoritySlot>,
        token: CodexAuthorityToken,
        control: Arc<CodexAuthorityControl>,
    ) -> Self {
        Self {
            slot,
            token,
            control,
        }
    }

    pub(in crate::worker::codex_hook) fn bind(self) -> CodexAuthorityRegistration {
        CodexAuthorityRegistration {
            slot: self.slot,
            token: self.token,
            control: self.control,
            installed: AtomicBool::new(false),
        }
    }
}

impl Drop for CodexAuthorityRegistration {
    fn drop(&mut self) {
        if !self.installed.load(Ordering::Acquire) {
            return;
        }
        let Ok(_takeover) = self.slot.takeover.lock() else {
            return;
        };
        let _ = self.slot.remove_exact(self.token, &self.control);
    }
}

impl CodexAuthorityRegistration {
    pub(in crate::worker::codex_hook) fn control(&self) -> &CodexAuthorityControl {
        &self.control
    }

    pub(super) fn control_arc(&self) -> &Arc<CodexAuthorityControl> {
        &self.control
    }

    pub(super) fn token(&self) -> CodexAuthorityToken {
        self.token
    }

    pub(super) fn succession_from(
        &self,
        _predecessor: CodexAuthorityToken,
    ) -> CodexSuccessionProof {
        CodexSuccessionProof { _private: () }
    }

    pub(super) fn mark_installed(&self) {
        self.installed.store(true, Ordering::Release);
    }
}
