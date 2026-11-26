use super::*;
use crate::pty_custody::tests::{descriptor_set, digest, request};

const IDENTITY_STEP: u64 = 1;
const PID_STEP: u32 = 1;
const SCHEMA_STEP: u16 = 1;
const OTHER_DIGEST_BYTE: u8 = 0x61;

fn fixture() -> (CustodyRequest, CustodyOffer, CustodyGuardianBinding) {
    let request = request();
    let offer = CustodyOffer::new(&request, descriptor_set(), digest());
    let guardian =
        CustodyGuardianBinding::new(Uuid::new_v4(), request.holder(), &request, &offer).unwrap();
    (request, offer, guardian)
}

#[test]
fn guardian_well_formedness_rejects_each_missing_identity() {
    let (request, offer, original) = fixture();
    let mutations: &[fn(&mut CustodyGuardianBinding)] = &[
        |value| value.schema_version += SCHEMA_STEP,
        |value| value.guardian_id = Uuid::nil(),
        |value| value.process.pid = u32::default(),
        |value| value.process.start_time = u64::default(),
        |value| value.host_holder.pid = u32::default(),
        |value| value.host_holder.start_time = u64::default(),
        |value| value.controller_generation = u64::default(),
    ];
    assert!(original.is_well_formed());
    for (index, mutate) in mutations.iter().enumerate() {
        let mut changed = original;
        mutate(&mut changed);
        assert!(!changed.is_well_formed(), "mutation {index}");
        assert!(!changed.matches(&request, &offer), "mutation {index}");
    }
}

#[test]
fn guardian_matches_only_its_exact_host_runtime_and_offer() {
    let (request, offer, original) = fixture();
    let mutations: &[fn(&mut CustodyGuardianBinding)] = &[
        |value| value.host_holder.pid += PID_STEP,
        |value| value.host_holder.start_time += IDENTITY_STEP,
        |value| value.runtime_identity_sha256 = Sha256Digest::repeated(OTHER_DIGEST_BYTE),
        |value| value.controller_generation += IDENTITY_STEP,
        |value| value.lease_id = CustodyLeaseId::new(),
        |value| value.offer_id = CustodyOfferId::new(),
        |value| value.descriptor_set = CustodyDescriptorSet::LinuxContainment,
        |value| value.profile = CustodyProfile::ActiveTerminalReadOnly,
        |value| value.recovery_identity_sha256 = Sha256Digest::repeated(OTHER_DIGEST_BYTE),
    ];
    assert!(original.matches(&request, &offer));
    for (index, mutate) in mutations.iter().enumerate() {
        let mut changed = original;
        mutate(&mut changed);
        assert!(changed.is_well_formed(), "mutation {index}");
        assert!(!changed.matches(&request, &offer), "mutation {index}");
    }
    let mut rejected = offer;
    rejected.lease_id = CustodyLeaseId::new();
    assert!(!original.matches(&request, &rejected));
    assert!(
        CustodyGuardianBinding::new(Uuid::new_v4(), request.holder(), &request, &rejected,)
            .is_none()
    );
}

#[test]
fn guardian_durable_lease_cannot_substitute_one_matching_field() {
    let (request, offer, guardian) = fixture();
    let pending = crate::pty_custody::CustodyDurableRecord::pending(request.runtime.clone());
    let prepared = pending
        .prepare_guardian(&request, &offer, guardian)
        .unwrap();
    let original = prepared.prepared_guardian_candidate().unwrap();
    let mutations: &[fn(&mut crate::pty_custody::CustodyDurableLease)] = &[
        |value| value.guardian = None,
        |value| value.holder.pid += PID_STEP,
        |value| value.holder.start_time += IDENTITY_STEP,
        |value| value.controller_generation += IDENTITY_STEP,
        |value| value.lease_id = CustodyLeaseId::new(),
        |value| value.offer_id = CustodyOfferId::new(),
        |value| value.descriptor_set = CustodyDescriptorSet::LinuxContainment,
        |value| value.profile = CustodyProfile::ActiveTerminalReadOnly,
        |value| value.recovery_identity_sha256 = Sha256Digest::repeated(OTHER_DIGEST_BYTE),
    ];
    assert!(guardian.matches_durable_lease(&request.runtime, &original));
    for (index, mutate) in mutations.iter().enumerate() {
        let mut changed = original;
        mutate(&mut changed);
        assert!(
            !guardian.matches_lease_identity(&changed),
            "mutation {index}"
        );
        assert!(
            !guardian.matches_durable_lease(&request.runtime, &changed),
            "mutation {index}"
        );
    }
    let mut wrong_runtime = request.runtime;
    wrong_runtime.runtime_epoch = Uuid::new_v4();
    assert!(!guardian.matches_durable_lease(&wrong_runtime, &original));
}

#[test]
fn rewriting_guardian_runtime_requires_complete_source_and_target() {
    let (request, _, guardian) = fixture();
    let mut target = request.runtime.clone();
    target.runtime_epoch = Uuid::new_v4();
    let rebound = guardian
        .rewrite_runtime_identity(&request.runtime, &target)
        .unwrap();
    assert!(rebound.matches_runtime(&target));
    assert!(!rebound.matches_runtime(&request.runtime));
    assert!(
        guardian
            .rewrite_runtime_identity(&target, &request.runtime)
            .is_none()
    );
    target.child_start_time = u64::default();
    assert!(!guardian.matches_runtime(&target));
    assert!(
        guardian
            .rewrite_runtime_identity(&request.runtime, &target)
            .is_none()
    );
}
