use super::linux_contract::BatchBudget;
use super::linux_io::{self, QueryValue};
use super::{LinuxPtyHolderState, PtySessionProof};
use crate::process::LiveProcessIdentity;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::process) enum LinuxPtyCustodyState {
    Target,
    Survivor,
    UntrustedHolder,
    Indeterminate,
}

pub(in crate::process) struct LinuxPtyCustodyScanner<'proof> {
    proof: &'proof PtySessionProof,
    budget: BatchBudget,
    expected_uid: u32,
}

impl<'proof> LinuxPtyCustodyScanner<'proof> {
    pub(in crate::process) fn new(proof: &'proof PtySessionProof) -> Self {
        Self {
            proof,
            budget: BatchBudget::new(),
            expected_uid: unsafe { libc::geteuid() },
        }
    }

    pub(in crate::process) fn classify(
        &mut self,
        identity: &LiveProcessIdentity,
    ) -> LinuxPtyCustodyState {
        match self
            .proof
            .linux_holder_state_with_budget(identity, &mut self.budget)
        {
            LinuxPtyHolderState::Excluded => LinuxPtyCustodyState::Survivor,
            LinuxPtyHolderState::Indeterminate => LinuxPtyCustodyState::Indeterminate,
            LinuxPtyHolderState::Holds => self.classify_holder(identity),
        }
    }

    fn classify_holder(&mut self, identity: &LiveProcessIdentity) -> LinuxPtyCustodyState {
        if !matches!(identity.try_matches(), Ok(true)) {
            return LinuxPtyCustodyState::Indeterminate;
        }
        if !matches!(
            linux_io::effective_uid(identity.pid()),
            QueryValue::Value(uid) if uid == self.expected_uid
        ) {
            return LinuxPtyCustodyState::UntrustedHolder;
        }
        let query = linux_io::environment_capability(
            identity.pid(),
            self.proof.environment_name(),
            self.proof.linux_capability(),
            self.budget.environment_window(),
        );
        self.budget.charge_environment(query.bytes);
        if !matches!(identity.try_matches(), Ok(true)) {
            return LinuxPtyCustodyState::Indeterminate;
        }
        match query.value {
            QueryValue::Value(true) => LinuxPtyCustodyState::Target,
            QueryValue::Value(false) => LinuxPtyCustodyState::UntrustedHolder,
            QueryValue::Pending(_) => LinuxPtyCustodyState::Indeterminate,
        }
    }
}
