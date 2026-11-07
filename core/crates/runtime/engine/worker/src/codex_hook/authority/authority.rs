use super::constants::{
    CODEX_AUTHORITY_EXHAUSTED_ERROR, CODEX_STORAGE_ERROR, CODEX_STORAGE_LATCHED_ERROR,
};
use super::publication::CodexHookPublication;
use crate::agent::hooks::{
    ApplyDisposition, HookEnvelope, HookIdentity, HookViolation, NormalizedLifecycle, Reducer,
};
use anyhow::{Context, Result, bail};

#[path = "authority_loss.rs"]
mod loss;
#[path = "authority_snapshot.rs"]
mod snapshot;
#[path = "authority_state.rs"]
mod state;
#[path = "authority_superseded.rs"]
mod superseded;
#[path = "authority_terminal.rs"]
mod terminal;
use loss::authority_lost_error;
pub(super) use loss::is_authority_lost_error;
pub(super) use state::AuthorityState;
pub(super) use superseded::is_superseded_error;
pub(super) use terminal::CommittedProcessExit;

pub(super) enum CommittedReduction {
    Accepted(CodexHookPublication),
    Rejected(HookViolation, NormalizedLifecycle),
}

pub(super) struct LifecycleAuthority {
    owner: HookIdentity,
    state: AuthorityState,
}
impl LifecycleAuthority {
    pub(super) fn apply_committed(&mut self, event: HookEnvelope) -> Result<CommittedReduction> {
        let (installed_revision, mut candidate) = self.candidate()?;
        let result = candidate.apply(event);
        let committed_state = self.install_candidate(installed_revision, candidate)?;
        match result {
            Ok(ApplyDisposition::Invalidated) => bail!(CODEX_STORAGE_ERROR),
            Ok(disposition) => Ok(CommittedReduction::Accepted(CodexHookPublication {
                disposition,
                state: committed_state,
            })),
            Err(violation) => Ok(CommittedReduction::Rejected(violation, committed_state)),
        }
    }

    fn candidate(&mut self) -> Result<(u64, Reducer)> {
        self.ensure_operable()?;
        let reducer = self.reducer()?;
        Ok((reducer.revision(), reducer.clone()))
    }

    fn install_candidate(
        &mut self,
        installed_revision: u64,
        candidate: Reducer,
    ) -> Result<NormalizedLifecycle> {
        if candidate.revision() == installed_revision {
            return Ok(candidate.state().clone());
        }
        let AuthorityState::Active { cursor, .. } = &self.state else {
            bail!(CODEX_STORAGE_LATCHED_ERROR);
        };
        let persisted = candidate
            .snapshot()
            .context(CODEX_STORAGE_ERROR)
            .and_then(|snapshot| cursor.persist(&snapshot).context(CODEX_STORAGE_ERROR));
        let cursor = match persisted {
            Ok(cursor) => cursor,
            Err(error) if crate::codex_hooks::is_snapshot_authority_changed(&error) => {
                return Err(error);
            }
            Err(error) => {
                self.latch_storage_failure();
                return Err(error);
            }
        };
        if candidate.authority_exhausted() {
            self.state = AuthorityState::Exhausted {
                reducer: candidate,
                cursor,
            };
            return Err(authority_exhausted_error());
        }
        let state = candidate.state().clone();
        self.state = AuthorityState::Active {
            reducer: candidate,
            cursor,
        };
        Ok(state)
    }
}

pub(super) fn is_authority_exhausted_error(error: &anyhow::Error) -> bool {
    error.downcast_ref::<HookViolation>() == Some(&HookViolation::AuthorityExhausted)
}

fn authority_exhausted_error() -> anyhow::Error {
    anyhow::Error::new(HookViolation::AuthorityExhausted).context(CODEX_AUTHORITY_EXHAUSTED_ERROR)
}
