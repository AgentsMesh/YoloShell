use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[path = "constants.rs"]
mod constants;
#[path = "control_state.rs"]
mod control_state;
#[path = "descriptor_set.rs"]
mod descriptor_set;
#[path = "durable_snapshot.rs"]
mod durable_snapshot;
#[path = "guardian/mod.rs"]
mod guardian;
#[path = "holder.rs"]
mod holder;
#[path = "identity.rs"]
mod identity;
#[path = "profile.rs"]
mod profile;
#[path = "protocol/mod.rs"]
mod protocol;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/reboot_test_hook.rs"]
mod reboot_test_hook;
#[path = "reboot_validation.rs"]
mod reboot_validation;
#[path = "record/mod.rs"]
mod record;
#[path = "release_settlement.rs"]
mod release_settlement;
#[path = "retirement_evidence.rs"]
mod retirement_evidence;
#[path = "transport.rs"]
mod transport;

pub use control_state::ExactCustodyReleasedAck;
pub use descriptor_set::{CustodyDescriptorSet, INVALID_PIPE_HANDLE};
pub use durable_snapshot::CustodyDurableSnapshot;
pub use guardian::binding::CustodyGuardianBinding;
pub use guardian::preparation::CustodyGuardianPreparation;
pub use holder::CustodyHolderObservation;
pub use holder::ExactCustodyHolderGone;
pub use identity::CustodyRuntimeIdentity;
pub use profile::CustodyProfile;
pub use protocol::state::ExactCustodyWorkerActivation;
pub use protocol::{CustodyControl, CustodyOffer};
pub use protocol::{
    CustodyControlAck, CustodyHostAck, CustodyRequest, CustodyWorkerActivated, CustodyWorkerCommit,
    CustodyWorkerReadyDisposition,
};
#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[doc(hidden)]
pub use reboot_test_hook::rewrite_boot_for_test;
pub use reboot_validation::{
    durable_record_digest, rewrite_boot_for_target, rewrite_boot_only,
    source_before_reboot_migration,
};
pub use record::CustodyDurableState;
pub use record::{CustodyDurableLease, CustodyDurableRecord};
#[cfg(any(test, feature = "runtime-component-tests"))]
#[path = "../test_support/record_guardian.rs"]
mod record_guardian_test_support;
pub use record::transition::{
    replace_exact as replace_exact_record, replace_exact_observed as replace_exact_record_observed,
    replace_exact_observed_with_fence as replace_exact_record_observed_with_fence,
};
#[cfg(any(test, feature = "runtime-component-tests"))]
pub use record_guardian_test_support::{arm_fixture, rearm_fixture};
pub use release_settlement::CustodyReleaseSettlement;
pub use retirement_evidence::CustodyRetirementEvidence;
pub use transport::CustodyUnixStream;

pub use constants::{GUARDIAN_PROTOCOL_VERSION, METHOD, PROTOCOL_VERSION};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CustodyLeaseId(#[serde(with = "crate::serde_non_nil_uuid")] Uuid);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CustodyOfferId(#[serde(with = "crate::serde_non_nil_uuid")] Uuid);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustodyHolderIdentity {
    pub pid: u32,
    pub start_time: u64,
}

impl CustodyLeaseId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl CustodyOfferId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/activation_tests.rs"]
mod activation_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/control_tests.rs"]
mod control_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/descriptor_set_tests.rs"]
mod descriptor_set_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/holder_tests.rs"]
mod holder_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/profile_tests.rs"]
mod profile_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/record_tests.rs"]
mod record_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/tests.rs"]
mod tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/transport_tests.rs"]
mod transport_tests;
