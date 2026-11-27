use super::super::authority_registry::CodexSuccessionProof;
use super::super::publication::{CodexAuthorityView, CodexLifecycleView};
use super::{LifecycleAuthority, authority_exhausted_error, authority_lost_error};
use crate::agent::hooks::{HookIdentity, NormalizedLifecycle, Reducer};
use crate::codex_hooks::{CodexSnapshotCursor, CodexSnapshotLineage};
use anyhow::{Result, bail};

pub(in crate::worker::codex_hook) enum AuthorityState {
    Active {
        reducer: Reducer,
        cursor: CodexSnapshotCursor,
    },
    Exhausted {
        reducer: Reducer,
        cursor: CodexSnapshotCursor,
    },
    AuthorityLost,
    StorageFailed,
    CommitIndeterminate,
    Superseded,
}

impl LifecycleAuthority {
    pub(in crate::worker::codex_hook) fn new(
        owner: HookIdentity,
        reducer: Reducer,
        cursor: CodexSnapshotCursor,
    ) -> Self {
        let state = if reducer.authority_exhausted() {
            AuthorityState::Exhausted { reducer, cursor }
        } else {
            AuthorityState::Active { reducer, cursor }
        };
        Self { owner, state }
    }

    pub(in crate::worker::codex_hook) fn owner(&self) -> &HookIdentity {
        &self.owner
    }

    pub(in crate::worker::codex_hook) fn reducer(&self) -> Result<&Reducer> {
        match &self.state {
            AuthorityState::Active { reducer, .. } | AuthorityState::Exhausted { reducer, .. } => {
                Ok(reducer)
            }
            AuthorityState::AuthorityLost => Err(authority_lost_error()),
            AuthorityState::StorageFailed | AuthorityState::CommitIndeterminate => {
                bail!(super::CODEX_STORAGE_LATCHED_ERROR)
            }
            AuthorityState::Superseded => Err(super::superseded::superseded_error()),
        }
    }

    pub(in crate::worker::codex_hook) fn lifecycle(&self) -> Option<NormalizedLifecycle> {
        self.reducer().ok().map(|reducer| reducer.state().clone())
    }

    pub(in crate::worker::codex_hook) fn storage_failed(&self) -> bool {
        matches!(
            self.state,
            AuthorityState::StorageFailed | AuthorityState::CommitIndeterminate
        )
    }

    pub(in crate::worker::codex_hook) fn commit_indeterminate(&self) -> bool {
        matches!(self.state, AuthorityState::CommitIndeterminate)
    }

    pub(in crate::worker::codex_hook) fn superseded(&self) -> bool {
        matches!(self.state, AuthorityState::Superseded)
    }

    pub(in crate::worker::codex_hook) fn authority_exhausted(&self) -> bool {
        matches!(self.state, AuthorityState::Exhausted { .. })
    }

    pub(in crate::worker::codex_hook) fn authority_view(&self) -> Option<CodexAuthorityView> {
        match &self.state {
            AuthorityState::Active { reducer, .. } => Some(CodexAuthorityView::Active(
                CodexLifecycleView::capture(reducer),
            )),
            AuthorityState::Exhausted { reducer, .. } => Some(CodexAuthorityView::Exhausted(
                CodexLifecycleView::capture(reducer),
            )),
            AuthorityState::AuthorityLost => Some(CodexAuthorityView::AuthorityLost),
            AuthorityState::StorageFailed | AuthorityState::CommitIndeterminate => None,
            AuthorityState::Superseded => Some(CodexAuthorityView::Superseded),
        }
    }

    pub(in crate::worker::codex_hook) fn snapshot_lineage(&self) -> Result<CodexSnapshotLineage> {
        match &self.state {
            AuthorityState::Active { cursor, .. } | AuthorityState::Exhausted { cursor, .. } => {
                cursor.snapshot_lineage()
            }
            AuthorityState::AuthorityLost => Err(authority_lost_error()),
            AuthorityState::StorageFailed | AuthorityState::CommitIndeterminate => {
                bail!(super::CODEX_STORAGE_LATCHED_ERROR)
            }
            AuthorityState::Superseded => Err(super::superseded::superseded_error()),
        }
    }

    pub(in crate::worker::codex_hook) fn latch_storage_failure(&mut self) -> bool {
        if matches!(
            self.state,
            AuthorityState::AuthorityLost
                | AuthorityState::StorageFailed
                | AuthorityState::CommitIndeterminate
                | AuthorityState::Superseded
        ) {
            return false;
        }
        self.state = AuthorityState::StorageFailed;
        true
    }

    pub(in crate::worker::codex_hook) fn latch_commit_indeterminate(&mut self) -> bool {
        if matches!(
            self.state,
            AuthorityState::AuthorityLost
                | AuthorityState::StorageFailed
                | AuthorityState::CommitIndeterminate
                | AuthorityState::Superseded
        ) {
            return false;
        }
        self.state = AuthorityState::CommitIndeterminate;
        true
    }

    pub(in crate::worker::codex_hook) fn latch_authority_lost(&mut self) -> bool {
        if matches!(
            self.state,
            AuthorityState::AuthorityLost
                | AuthorityState::StorageFailed
                | AuthorityState::CommitIndeterminate
                | AuthorityState::Superseded
        ) {
            return false;
        }
        self.state = AuthorityState::AuthorityLost;
        true
    }

    pub(in crate::worker::codex_hook) fn supersede_authorized(
        &mut self,
        _proof: &CodexSuccessionProof,
    ) {
        self.state = AuthorityState::Superseded;
    }

    pub(in crate::worker::codex_hook) fn ensure_operable(&mut self) -> Result<()> {
        match &self.state {
            AuthorityState::AuthorityLost => return Err(authority_lost_error()),
            AuthorityState::StorageFailed | AuthorityState::CommitIndeterminate => {
                bail!(super::CODEX_STORAGE_LATCHED_ERROR)
            }
            AuthorityState::Superseded => return Err(super::superseded::superseded_error()),
            AuthorityState::Exhausted { .. } => return Err(authority_exhausted_error()),
            AuthorityState::Active { reducer, .. } if reducer.authority_exhausted() => {}
            AuthorityState::Active { .. } => return Ok(()),
        }
        let state = std::mem::replace(&mut self.state, AuthorityState::StorageFailed);
        let AuthorityState::Active { reducer, cursor } = state else {
            unreachable!();
        };
        self.state = AuthorityState::Exhausted { reducer, cursor };
        Err(authority_exhausted_error())
    }

    #[cfg(test)]
    pub(in crate::worker::codex_hook) fn exhaust_active_authority_for_test(&mut self) {
        let AuthorityState::Active { reducer, .. } = &mut self.state else {
            panic!("test authority must be active");
        };
        reducer.exhaust_active_authority_for_test();
    }
}
