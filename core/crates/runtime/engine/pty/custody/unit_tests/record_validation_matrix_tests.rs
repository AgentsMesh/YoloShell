use super::*;
use crate::pty_custody::tests::{descriptor_set, digest, request};
use crate::pty_custody::{
    CustodyDescriptorSet, CustodyGuardianBinding, CustodyOffer, CustodyProfile,
};
use uuid::Uuid;

const GENERATION_STEP: u64 = 1;

#[test]
fn each_lease_identity_and_platform_binding_is_independently_required() {
    let request = request();
    let offer = CustodyOffer::new(&request, descriptor_set(), digest());
    let binding =
        CustodyGuardianBinding::new(Uuid::new_v4(), request.holder(), &request, &offer).unwrap();
    let prepared = CustodyDurableRecord::pending(request.runtime.clone())
        .prepare_guardian(&request, &offer, binding)
        .unwrap();
    let original = prepared.prepared_guardian_candidate().unwrap();
    let mutations: &[fn(&mut CustodyDurableLease)] = &[
        |value| value.holder.pid = u32::default(),
        |value| value.holder.start_time = u64::default(),
        |value| value.controller_generation = u64::default(),
        |value| {
            value.descriptor_set = CustodyDescriptorSet::macos_process_capability(
                crate::pty_custody::INVALID_PIPE_HANDLE,
                crate::pty_custody::INVALID_PIPE_HANDLE,
            )
        },
        |value| value.profile = CustodyProfile::ReleasedTerminal,
        |value| value.guardian = None,
        |value| value.descriptor_set = CustodyDescriptorSet::LinuxContainment,
    ];
    assert!(lease_is_valid(&original, &request.runtime));
    for (index, mutate) in mutations.iter().enumerate() {
        let mut changed = original;
        mutate(&mut changed);
        assert!(
            !lease_is_valid(&changed, &request.runtime),
            "mutation {index}"
        );
    }
    for (descriptors, profile) in [
        (
            CustodyDescriptorSet::LinuxContainment,
            CustodyProfile::ActiveRunning,
        ),
        (
            CustodyDescriptorSet::ReleasedSuccessor,
            CustodyProfile::ReleasedTerminal,
        ),
    ] {
        let mut alternate = original;
        alternate.guardian = None;
        alternate.descriptor_set = descriptors;
        alternate.profile = profile;
        assert!(lease_is_valid(&alternate, &request.runtime));
        alternate.guardian = Some(binding);
        assert!(!lease_is_valid(&alternate, &request.runtime));
    }
}

#[test]
fn preparation_requires_a_new_generation_lease_and_offer() {
    let request = request();
    let offer = CustodyOffer::new(&request, descriptor_set(), digest());
    let pending = CustodyDurableRecord::pending(request.runtime.clone());
    let armed = crate::pty_custody::arm_fixture(&pending, &request, &offer).unwrap();
    let predecessor = armed.armed_lease().unwrap();
    let mut next_request = request.clone();
    next_request.controller_generation += GENERATION_STEP;
    next_request.lease_id = crate::pty_custody::CustodyLeaseId::new();
    let next_offer = CustodyOffer::new(&next_request, descriptor_set(), digest());
    let next_guardian =
        CustodyGuardianBinding::new(Uuid::new_v4(), request.holder(), &next_request, &next_offer)
            .unwrap();
    let prepared = armed
        .prepare_guardian(&next_request, &next_offer, next_guardian)
        .unwrap();
    let candidate = prepared.prepared_guardian_candidate().unwrap();
    assert!(preparation_is_valid(
        &armed.state,
        &candidate,
        &request.runtime
    ));
    for (generation, lease, offer_id) in [
        (
            predecessor.controller_generation,
            next_request.lease_id,
            next_offer.offer_id,
        ),
        (
            next_request.controller_generation,
            predecessor.lease_id,
            next_offer.offer_id,
        ),
        (
            next_request.controller_generation,
            next_request.lease_id,
            predecessor.offer_id,
        ),
    ] {
        let mut changed_request = next_request.clone();
        changed_request.controller_generation = generation;
        changed_request.lease_id = lease;
        let mut changed_offer = CustodyOffer::new(&changed_request, descriptor_set(), digest());
        changed_offer.offer_id = offer_id;
        let guardian = CustodyGuardianBinding::new(
            Uuid::new_v4(),
            request.holder(),
            &changed_request,
            &changed_offer,
        )
        .unwrap();
        let candidate = pending
            .prepare_guardian(&changed_request, &changed_offer, guardian)
            .unwrap()
            .prepared_guardian_candidate()
            .unwrap();
        assert!(lease_is_valid(&candidate, &request.runtime));
        assert!(!preparation_is_valid(
            &armed.state,
            &candidate,
            &request.runtime
        ));
        assert!(
            armed
                .prepare_guardian(&changed_request, &changed_offer, guardian)
                .is_none()
        );
    }
    assert!(!preparation_is_valid(
        &prepared.state,
        &candidate,
        &request.runtime
    ));
    let retired = armed.retire(predecessor).unwrap();
    assert!(!preparation_is_valid(
        &retired.state,
        &candidate,
        &request.runtime
    ));
}
