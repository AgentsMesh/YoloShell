use super::boot_identity::BootIdentity;
use super::{BootIdentityRelation, ProcessIdentity, ProcessIdentityObservation};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExactProcessGone {
    identity: ProcessIdentity,
    boot: BootIdentity,
}

impl ExactProcessGone {
    pub fn capture(identity: ProcessIdentity) -> Option<Self> {
        Self::capture_observed(identity, ProcessIdentity::observe)
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    #[doc(hidden)]
    pub fn capture_for_test(
        identity: ProcessIdentity,
        observation: ProcessIdentityObservation,
    ) -> Option<Self> {
        Self::capture_observed(identity, |_| observation)
    }

    pub fn identity(self) -> ProcessIdentity {
        self.identity
    }

    pub fn boot_identity(self) -> BootIdentity {
        self.boot
    }

    fn capture_observed(
        identity: ProcessIdentity,
        observe: impl FnOnce(ProcessIdentity) -> ProcessIdentityObservation,
    ) -> Option<Self> {
        let boot = BootIdentity::current();
        let relation = boot.relation_to_current();
        Self::capture_with(identity, boot, relation, observe)
    }

    fn capture_with(
        identity: ProcessIdentity,
        boot: BootIdentity,
        relation: BootIdentityRelation,
        observe: impl FnOnce(ProcessIdentity) -> ProcessIdentityObservation,
    ) -> Option<Self> {
        if !valid_identity(identity) || !boot.known() || relation != BootIdentityRelation::Same {
            return None;
        }
        (observe(identity) == ProcessIdentityObservation::Gone).then_some(Self { identity, boot })
    }
}

fn valid_identity(identity: ProcessIdentity) -> bool {
    identity.pid != u32::default()
        && i32::try_from(identity.pid).is_ok()
        && identity.start_time != u64::default()
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/exact_process_gone_tests.rs"]
mod tests;
