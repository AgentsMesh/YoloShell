use super::super::constants::{
    LINUX_PTY_PROOF_ENVIRONMENT_BYTES, LINUX_PTY_PROOF_FILE_DESCRIPTORS,
    LINUX_PTY_PROOF_TOTAL_ENVIRONMENT_BYTES, LINUX_PTY_PROOF_TOTAL_FILE_DESCRIPTORS,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CandidateProof {
    Excluded,
    Matched,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CandidateEvaluation {
    Complete(CandidateProof),
    Pending(CandidatePending),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CandidatePending {
    Exited,
    Permission(QueryKind),
    Budget(BudgetKind),
    Query(QueryKind),
    IdentityChanged,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum QueryKind {
    Identity,
    Status,
    Environment,
    Descriptors,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BudgetKind {
    RetainedAuthorities,
    CandidateStatus,
    CandidateEnvironment,
    BatchEnvironment,
    CandidateDescriptors,
    BatchDescriptors,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct WorkWindow {
    pub(super) limit: usize,
    pub(super) exhausted: BudgetKind,
}

pub(super) struct BatchBudget {
    environment: usize,
    descriptors: usize,
}

impl BatchBudget {
    pub(super) fn new() -> Self {
        Self {
            environment: LINUX_PTY_PROOF_TOTAL_ENVIRONMENT_BYTES,
            descriptors: LINUX_PTY_PROOF_TOTAL_FILE_DESCRIPTORS,
        }
    }

    pub(super) fn environment_window(&self) -> WorkWindow {
        window(
            self.environment,
            LINUX_PTY_PROOF_ENVIRONMENT_BYTES,
            BudgetKind::BatchEnvironment,
            BudgetKind::CandidateEnvironment,
        )
    }

    pub(super) fn descriptor_window(&self) -> WorkWindow {
        window(
            self.descriptors,
            LINUX_PTY_PROOF_FILE_DESCRIPTORS,
            BudgetKind::BatchDescriptors,
            BudgetKind::CandidateDescriptors,
        )
    }

    pub(super) fn charge_environment(&mut self, count: usize) {
        self.environment = self.environment.saturating_sub(count);
    }

    pub(super) fn charge_descriptors(&mut self, count: usize) {
        self.descriptors = self.descriptors.saturating_sub(count);
    }
}

fn window(remaining: usize, candidate: usize, batch: BudgetKind, item: BudgetKind) -> WorkWindow {
    WorkWindow {
        limit: remaining.min(candidate),
        exhausted: (remaining < candidate).then_some(batch).unwrap_or(item),
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/linux/pty_session_linux_contract_tests.rs"]
mod tests;
