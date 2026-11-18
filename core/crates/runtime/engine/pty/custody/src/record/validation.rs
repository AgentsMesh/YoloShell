use crate::pty_custody::record::CUSTODY_RECORD_SCHEMA_VERSION;
use crate::pty_custody::{CustodyDurableLease, CustodyDurableRecord, CustodyDurableState};

impl CustodyDurableRecord {
    pub fn is_valid(&self) -> bool {
        self.runtime.is_complete()
            && self.schema_version == CUSTODY_RECORD_SCHEMA_VERSION
            && state_is_valid(&self.state, &self.runtime, true)
    }
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/record_validation_matrix_tests.rs"]
mod matrix_tests;

fn state_is_valid(
    state: &CustodyDurableState,
    runtime: &crate::pty_custody::CustodyRuntimeIdentity,
    preparation_allowed: bool,
) -> bool {
    match state {
        CustodyDurableState::Pending {} => true,
        CustodyDurableState::PreparedGuardian {
            predecessor,
            candidate,
        } => preparation_allowed && preparation_is_valid(predecessor, candidate, runtime),
        CustodyDurableState::Armed { .. } => {
            state_lease(state).is_some_and(|lease| lease_is_valid(&lease, runtime))
        }
        CustodyDurableState::Retired {
            retirement_evidence,
            ..
        } => state_lease(state).is_some_and(|lease| {
            lease_is_valid(&lease, runtime)
                && retirement_evidence.is_none_or(|evidence| evidence.is_valid())
        }),
    }
}

fn preparation_is_valid(
    predecessor: &CustodyDurableState,
    candidate: &CustodyDurableLease,
    runtime: &crate::pty_custody::CustodyRuntimeIdentity,
) -> bool {
    state_is_valid(predecessor, runtime, false)
        && lease_is_valid(candidate, runtime)
        && match predecessor {
            CustodyDurableState::Pending {} => true,
            CustodyDurableState::Armed { .. } => state_lease(predecessor).is_some_and(|lease| {
                candidate.controller_generation > lease.controller_generation
                    && candidate.lease_id != lease.lease_id
                    && candidate.offer_id != lease.offer_id
            }),
            CustodyDurableState::PreparedGuardian { .. } | CustodyDurableState::Retired { .. } => {
                false
            }
        }
}

fn lease_is_valid(
    lease: &CustodyDurableLease,
    runtime: &crate::pty_custody::CustodyRuntimeIdentity,
) -> bool {
    lease_fields_are_valid(lease)
        && match (lease.descriptor_set, lease.guardian) {
            (
                crate::pty_custody::CustodyDescriptorSet::MacosProcessCapability { .. },
                Some(binding),
            ) => binding.is_well_formed() && binding.matches_durable_lease(runtime, lease),
            (
                crate::pty_custody::CustodyDescriptorSet::LinuxContainment
                | crate::pty_custody::CustodyDescriptorSet::ReleasedSuccessor,
                None,
            ) => true,
            _ => false,
        }
}

fn lease_fields_are_valid(lease: &CustodyDurableLease) -> bool {
    lease.holder.pid != u32::default()
        && lease.holder.start_time != u64::default()
        && lease.controller_generation != u64::default()
        && lease.descriptor_set.is_well_formed()
        && lease.profile.accepts_descriptor_set(lease.descriptor_set)
}

fn state_lease(state: &CustodyDurableState) -> Option<CustodyDurableLease> {
    match *state {
        CustodyDurableState::Armed {
            lease_id,
            offer_id,
            holder,
            controller_generation,
            descriptor_set,
            profile,
            recovery_identity_sha256,
            guardian,
        }
        | CustodyDurableState::Retired {
            lease_id,
            offer_id,
            holder,
            controller_generation,
            descriptor_set,
            profile,
            recovery_identity_sha256,
            guardian,
            ..
        } => Some(CustodyDurableLease {
            lease_id,
            offer_id,
            holder,
            controller_generation,
            descriptor_set,
            profile,
            recovery_identity_sha256,
            guardian,
        }),
        CustodyDurableState::Pending {} | CustodyDurableState::PreparedGuardian { .. } => None,
    }
}
