use super::PtySessionProof;
use super::linux_contract::{
    BatchBudget, CandidateEvaluation, CandidatePending, CandidateProof, QueryKind,
};
use super::linux_fds::{self, DescriptorValue};
use super::linux_io::{self, QueryValue};
use crate::process::LiveProcessIdentity;

impl PtySessionProof {
    pub(super) fn evaluate_linux_candidate(
        &self,
        candidate: &LiveProcessIdentity,
        expected_uid: u32,
        budget: &mut BatchBudget,
    ) -> CandidateEvaluation {
        if let Some(pending) = identity_pending(candidate) {
            return CandidateEvaluation::Pending(pending);
        }
        if let Err(pending) = validate_uid(linux_io::effective_uid(candidate.pid()), expected_uid) {
            return finish(
                candidate,
                expected_uid,
                CandidateEvaluation::Pending(pending),
            );
        }
        let environment = linux_io::environment_capability(
            candidate.pid(),
            self.environment_name(),
            &self.capability,
            budget.environment_window(),
        );
        budget.charge_environment(environment.bytes);
        let evaluation = match environment.value {
            QueryValue::Value(false) => CandidateEvaluation::Complete(CandidateProof::Excluded),
            QueryValue::Value(true) if self.is_discovery() => {
                CandidateEvaluation::Complete(CandidateProof::Matched)
            }
            QueryValue::Value(true) => self.evaluate_linux_tty(candidate.pid(), budget, None),
            QueryValue::Pending(pending) => {
                if self.is_discovery() {
                    CandidateEvaluation::Pending(pending)
                } else {
                    self.evaluate_linux_tty(candidate.pid(), budget, Some(pending))
                }
            }
        };
        finish(candidate, expected_uid, evaluation)
    }

    fn evaluate_linux_tty(
        &self,
        pid: u32,
        budget: &mut BatchBudget,
        environment_pending: Option<CandidatePending>,
    ) -> CandidateEvaluation {
        let descriptors = linux_fds::holds_device(pid, self.tty_device, budget.descriptor_window());
        budget.charge_descriptors(descriptors.examined);
        match (environment_pending, descriptors.value) {
            (None, DescriptorValue::Holds) => {
                CandidateEvaluation::Complete(CandidateProof::Matched)
            }
            (_, DescriptorValue::Excluded) => {
                CandidateEvaluation::Complete(CandidateProof::Excluded)
            }
            (Some(pending), DescriptorValue::Holds) => CandidateEvaluation::Pending(pending),
            (_, DescriptorValue::Pending(pending)) => CandidateEvaluation::Pending(pending),
        }
    }
}

fn finish(
    candidate: &LiveProcessIdentity,
    expected_uid: u32,
    evaluation: CandidateEvaluation,
) -> CandidateEvaluation {
    let uid = linux_io::effective_uid(candidate.pid());
    if let Some(pending) = identity_pending(candidate) {
        return CandidateEvaluation::Pending(pending);
    }
    if let Err(pending) = validate_uid(uid, expected_uid) {
        return CandidateEvaluation::Pending(pending);
    }
    evaluation
}

fn identity_pending(candidate: &LiveProcessIdentity) -> Option<CandidatePending> {
    match candidate.try_matches() {
        Ok(true) => None,
        Ok(false) => Some(CandidatePending::Exited),
        Err(_) => Some(CandidatePending::Query(QueryKind::Identity)),
    }
}

fn validate_uid(value: QueryValue<u32>, expected: u32) -> Result<(), CandidatePending> {
    match value {
        QueryValue::Value(actual) if actual == expected => Ok(()),
        QueryValue::Value(_) => Err(CandidatePending::IdentityChanged),
        QueryValue::Pending(pending) => Err(pending),
    }
}
