use crate::constants::{AUTHORITY_REVISION_STEP, INITIAL_AUTHORITY_REVISION};
use crate::snapshot::SnapshotViolation;
use crate::state::{NormalizedLifecycle, ProcessState};
use crate::violation::HookViolation;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AuthorityRevision(u64);

impl AuthorityRevision {
    pub(crate) const fn initial() -> Self {
        Self(INITIAL_AUTHORITY_REVISION)
    }

    pub(crate) fn restore(value: u64) -> Result<Self, SnapshotViolation> {
        if value == 0 || value == u64::MAX {
            return Err(SnapshotViolation::InvalidRevision);
        }
        Ok(Self(value))
    }

    pub(crate) const fn get(self) -> u64 {
        self.0
    }

    pub(crate) fn validate_for(self, state: &NormalizedLifecycle) -> Result<(), SnapshotViolation> {
        let active = state
            .last_sequence
            .0
            .checked_add(INITIAL_AUTHORITY_REVISION)
            .ok_or(SnapshotViolation::InvalidRevision)?;
        let terminal = active
            .checked_add(AUTHORITY_REVISION_STEP)
            .filter(|revision| *revision != u64::MAX);
        let post_exit_revocation = terminal
            .and_then(|revision| revision.checked_add(AUTHORITY_REVISION_STEP))
            .filter(|revision| *revision != u64::MAX);
        let valid = if state.revoked {
            terminal == Some(self.0) || post_exit_revocation == Some(self.0)
        } else if state.process == ProcessState::Exited {
            terminal == Some(self.0)
        } else {
            active == self.0
        };
        valid
            .then_some(())
            .ok_or(SnapshotViolation::InvalidRevision)
    }

    pub(crate) fn advance(&mut self) -> Result<(), HookViolation> {
        let next = self
            .0
            .checked_add(AUTHORITY_REVISION_STEP)
            .filter(|revision| *revision != u64::MAX)
            .ok_or(HookViolation::AuthorityExhausted)?;
        self.0 = next;
        Ok(())
    }

    pub(crate) fn is_exhausted(self) -> bool {
        self.0
            .checked_add(AUTHORITY_REVISION_STEP)
            .filter(|revision| *revision != u64::MAX)
            .is_none()
    }

    #[cfg(any(test, feature = "yolo-test-support"))]
    pub(crate) const fn last_usable() -> Self {
        Self(u64::MAX - AUTHORITY_REVISION_STEP)
    }

    #[cfg(any(test, feature = "yolo-test-support"))]
    pub(crate) const fn penultimate() -> Self {
        Self(Self::last_usable().0 - AUTHORITY_REVISION_STEP)
    }
}
