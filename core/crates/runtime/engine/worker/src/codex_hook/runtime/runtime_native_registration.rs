use super::CodexHookRuntime;
use crate::agent::hooks::{EventSequence, FIRST_SEQUENCE};
use crate::codex_hooks::CodexAdmissionAuthority;
use anyhow::{Result, ensure};

const REGISTRATION_CONFLICT: &str =
    "Codex registration settlement does not match its exact first event";

impl CodexHookRuntime {
    pub(in crate::worker::codex_hook) fn complete_native_registration(
        &self,
        admission: &CodexAdmissionAuthority,
    ) -> Result<()> {
        admission.verify()?;
        let first = admission.record().first_event();
        ensure!(first.identity == self.owner, REGISTRATION_CONFLICT);
        let mut lifecycle = self.lock_lifecycle_anyhow()?;
        lifecycle.ensure_operable()?;
        let snapshot = lifecycle.prove_snapshot_current()?;
        let pending = self.outbox.with_locked_state(|state| {
            super::super::replay::validate_restored(&lifecycle, state)?;
            let mut expected = first.clone();
            expected.sequence = EventSequence(FIRST_SEQUENCE);
            if state.acknowledged_sequence() >= FIRST_SEQUENCE {
                if let Some(retained) = state
                    .retained_events()
                    .find(|event| event.sequence == EventSequence::FIRST)
                {
                    ensure!(retained == &expected, REGISTRATION_CONFLICT);
                }
                return Ok(false);
            }
            ensure!(
                !lifecycle.reducer()?.is_revoked()
                    && state.pending_head() == Some(&expected)
                    && lifecycle.reducer()?.state().last_sequence >= EventSequence::FIRST,
                REGISTRATION_CONFLICT
            );
            Ok(true)
        })?;
        if pending {
            if let Err(error) = self.outbox.acknowledge(FIRST_SEQUENCE, first.event_id) {
                if crate::codex_hooks::is_outbox_commit_indeterminate(&error) {
                    self.latch_commit_indeterminate_locked(&mut lifecycle);
                } else {
                    lifecycle.latch_storage_failure();
                    self.publish_storage_unavailable();
                }
                return Err(error);
            }
        }
        snapshot.linearize_publication()?;
        admission.verify()
    }
}
