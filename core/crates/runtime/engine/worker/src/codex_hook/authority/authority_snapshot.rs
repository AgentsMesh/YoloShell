use super::{AuthorityState, LifecycleAuthority};
use crate::codex_hooks::{CodexSnapshotClaimToken, CodexSnapshotProof};
use anyhow::{Context, Result, bail};

impl LifecycleAuthority {
    pub(in crate::worker::codex_hook) fn prove_snapshot_current(
        &self,
    ) -> Result<CodexSnapshotProof> {
        let cursor = match &self.state {
            AuthorityState::Active { cursor, .. } | AuthorityState::Exhausted { cursor, .. } => {
                cursor
            }
            AuthorityState::AuthorityLost => return Err(super::authority_lost_error()),
            AuthorityState::StorageFailed | AuthorityState::CommitIndeterminate => {
                bail!(super::CODEX_STORAGE_LATCHED_ERROR)
            }
            AuthorityState::Superseded => return Err(super::superseded::superseded_error()),
        };
        cursor.prove_current().context(super::CODEX_STORAGE_ERROR)
    }

    pub(in crate::worker::codex_hook) fn claim_snapshot_with(
        &mut self,
        publish: impl FnOnce(&CodexSnapshotProof) -> Result<()>,
    ) -> Result<CodexSnapshotClaimToken> {
        let (reducer, cursor) = match &mut self.state {
            AuthorityState::Active { reducer, cursor }
            | AuthorityState::Exhausted { reducer, cursor } => (reducer, cursor),
            AuthorityState::AuthorityLost => return Err(super::authority_lost_error()),
            AuthorityState::StorageFailed | AuthorityState::CommitIndeterminate => {
                bail!(super::CODEX_STORAGE_LATCHED_ERROR)
            }
            AuthorityState::Superseded => return Err(super::superseded::superseded_error()),
        };
        let snapshot = reducer.snapshot().context(super::CODEX_STORAGE_ERROR)?;
        let claim = cursor
            .claim(&snapshot)
            .context(super::CODEX_STORAGE_ERROR)?;
        claim
            .publish_with(publish)
            .context(super::CODEX_STORAGE_ERROR)
    }
}
