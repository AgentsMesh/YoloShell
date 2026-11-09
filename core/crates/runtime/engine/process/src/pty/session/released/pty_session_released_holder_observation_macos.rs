use std::fmt::Display;

const HOLDER_QUERY_OBSERVATION_LOG: &str = "released PTY path-holder priority query observed";
const HOLDER_QUERY_FAILURE_LOG: &str =
    "released PTY path-holder priority query failed; full inventory scan remains active";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::process::pty_session_proof) enum ReleasedHolderQueryStatus {
    #[default]
    NotAttempted,
    Succeeded,
    MissingPath,
    Failed,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::process::pty_session_proof) struct ReleasedHolderQueryObservation {
    pub(in crate::process::pty_session_proof) status: ReleasedHolderQueryStatus,
    pub(in crate::process::pty_session_proof) holder_pid_count: usize,
    pub(in crate::process::pty_session_proof) matched_identity_count: usize,
}

impl ReleasedHolderQueryObservation {
    pub(super) fn succeeded(holder_pid_count: usize, matched_identity_count: usize) -> Self {
        Self {
            status: ReleasedHolderQueryStatus::Succeeded,
            holder_pid_count,
            matched_identity_count,
        }
    }

    pub(super) fn failed() -> Self {
        Self {
            status: ReleasedHolderQueryStatus::Failed,
            ..Self::default()
        }
    }

    pub(super) fn missing_path() -> Self {
        Self {
            status: ReleasedHolderQueryStatus::MissingPath,
            ..Self::default()
        }
    }

    pub(super) fn path_is_missing(self) -> bool {
        self.status == ReleasedHolderQueryStatus::MissingPath
    }

    pub(super) fn covers_inventory(self) -> bool {
        matches!(
            self.status,
            ReleasedHolderQueryStatus::Succeeded | ReleasedHolderQueryStatus::MissingPath
        ) && self.holder_pid_count == self.matched_identity_count
    }

    pub(super) fn trace(self, inventory_candidate_count: usize) {
        tracing::trace!(
            query_status = ?self.status,
            self.holder_pid_count,
            self.matched_identity_count,
            inventory_candidate_count,
            message = HOLDER_QUERY_OBSERVATION_LOG,
        );
    }

    pub(super) fn warn_failure_transition(
        self,
        previous: Self,
        inventory_candidate_count: usize,
        error: &impl Display,
    ) {
        self.trace(inventory_candidate_count);
        if previous.status != ReleasedHolderQueryStatus::Failed {
            tracing::warn!(
                %error,
                inventory_candidate_count,
                message = HOLDER_QUERY_FAILURE_LOG,
            );
        }
    }
}
