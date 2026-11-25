use super::super::authority::{
    LifecycleAuthority, is_authority_exhausted_error, is_authority_lost_error, is_superseded_error,
};
use super::super::authority_control::CodexAuthorityControl;
use super::super::authority_registry::CodexSuccessionProof;
use super::super::error::CodexHookRejection;
use super::super::publication::{CodexAuthorityView, CodexLifecycleView};
use super::CodexHookRuntime;
use crate::agent::hooks::HookViolation;
use crate::codex_hooks::CodexSnapshotProof;
use anyhow::{Context, Result};

impl CodexHookRuntime {
    pub(super) fn contain_authority_error(
        &self,
        lifecycle: &mut LifecycleAuthority,
        error: anyhow::Error,
        proof: Option<&CodexSnapshotProof>,
    ) -> anyhow::Error {
        if lifecycle.superseded() || is_superseded_error(&error) {
            return error;
        }
        if is_authority_lost_error(&error) {
            if lifecycle.latch_authority_lost() {
                self.publish_authority_lost();
            }
        } else if is_authority_exhausted_error(&error) && lifecycle.authority_exhausted() {
            let Some(proof) = proof else {
                if lifecycle.latch_storage_failure() {
                    self.publish_storage_unavailable();
                }
                return error;
            };
            if let Err(publication_error) = self.publish_lifecycle(proof, lifecycle) {
                return self.contain_authority_error(lifecycle, publication_error, None);
            }
        } else {
            if lifecycle.latch_storage_failure() {
                self.publish_storage_unavailable();
            }
        }
        error
    }

    pub(super) fn reject_authority_error(
        &self,
        lifecycle: &mut LifecycleAuthority,
        error: anyhow::Error,
        proof: Option<&CodexSnapshotProof>,
    ) -> CodexHookRejection {
        let error = self.contain_authority_error(lifecycle, error, proof);
        if is_superseded_error(&error) || lifecycle.superseded() {
            CodexHookRejection::Superseded
        } else if is_authority_lost_error(&error) {
            CodexHookRejection::AuthorityLost(error)
        } else if is_authority_exhausted_error(&error) && lifecycle.authority_exhausted() {
            CodexHookRejection::Violation(HookViolation::AuthorityExhausted)
        } else {
            CodexHookRejection::Storage(error)
        }
    }

    pub(super) fn publish_lifecycle(
        &self,
        proof: &CodexSnapshotProof,
        lifecycle: &LifecycleAuthority,
    ) -> Result<()> {
        if let Some(view) = lifecycle.authority_view() {
            return self.registration.control().publish_proven(proof, view);
        }
        Ok(())
    }

    pub(super) fn publish_active(
        &self,
        proof: &CodexSnapshotProof,
        state: CodexLifecycleView,
    ) -> Result<()> {
        self.registration
            .control()
            .publish_proven(proof, CodexAuthorityView::Active(state))
    }

    fn publish_authority_lost(&self) {
        self.registration
            .control()
            .publish_terminal_authority_lost();
    }

    pub(super) fn publish_storage_unavailable(&self) {
        self.registration
            .control()
            .publish_terminal_storage_unavailable();
    }
}

impl CodexAuthorityControl {
    pub(super) fn publish_initial(
        &self,
        proof: &CodexSnapshotProof,
        view: CodexAuthorityView,
    ) -> Result<()> {
        self.publish_proven(proof, view)
    }

    pub(super) fn publish_proven(
        &self,
        proof: &CodexSnapshotProof,
        view: CodexAuthorityView,
    ) -> Result<()> {
        proof
            .linearize_publication()
            .context(super::super::constants::CODEX_STORAGE_ERROR)?;
        self.authority_tx.send_replace(view);
        Ok(())
    }

    pub(super) fn publish_terminal_authority_lost(&self) {
        self.authority_tx.send_if_modified(|view| {
            if matches!(
                view,
                CodexAuthorityView::AuthorityLost | CodexAuthorityView::Superseded
            ) {
                return false;
            }
            *view = CodexAuthorityView::AuthorityLost;
            true
        });
    }

    pub(in crate::worker::codex_hook) fn publish_terminal_storage_unavailable(&self) {
        self.authority_tx.send_if_modified(|view| {
            if matches!(
                view,
                CodexAuthorityView::AuthorityLost
                    | CodexAuthorityView::StorageUnavailable
                    | CodexAuthorityView::Superseded
            ) {
                return false;
            }
            *view = CodexAuthorityView::StorageUnavailable;
            true
        });
    }

    pub(super) fn supersede_locked(
        &self,
        lifecycle: &mut LifecycleAuthority,
        proof: CodexSuccessionProof,
    ) {
        lifecycle.supersede_authorized(&proof);
        self.publish_superseded(proof);
    }

    pub(super) fn contain_failed_takeover(&self, lifecycle: &mut LifecycleAuthority) {
        if lifecycle.prove_snapshot_current().is_ok() || !lifecycle.latch_authority_lost() {
            return;
        }
        self.publish_terminal_authority_lost();
    }

    fn publish_superseded(&self, _proof: CodexSuccessionProof) {
        self.authority_tx
            .send_replace(CodexAuthorityView::Superseded);
    }
}
