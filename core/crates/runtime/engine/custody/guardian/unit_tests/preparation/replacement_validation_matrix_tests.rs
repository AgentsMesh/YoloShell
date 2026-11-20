use super::*;
use crate::custody_guardian::bootstrap_validation::{
    validate_prepared_successor, validate_successor,
};
use crate::custody_guardian::protocol::GuardianBootstrap;
use crate::pty_custody::{
    CustodyDescriptorSet, CustodyDurableRecord, CustodyDurableSnapshot, CustodyDurableState,
    CustodyGuardianBinding, CustodyLeaseId, CustodyProfile, CustodyRequest,
};
use uuid::Uuid;

const OTHER_DIGEST: u8 = 0x73;
const GENERATION_STEP: u64 = 1;

#[test]
fn guardian_successor_rejects_each_lease_generation_and_descriptor_substitution() {
    let fixture = Fixture::new(ProcessIdentityRelation::Gone);
    let original = fixture.snapshot.record().armed_lease().unwrap();
    validate_successor(&fixture.bootstrap, &fixture.runtime, original).unwrap();
    let mutations: &[fn(&mut GuardianBootstrap)] = &[
        |value| value.request.controller_generation -= GENERATION_STEP,
        |value| value.offer.descriptor_set = CustodyDescriptorSet::LinuxContainment,
        |value| {
            value.offer.recovery_identity_sha256 =
                crate::sha256_digest::Sha256Digest::repeated(OTHER_DIGEST)
        },
        |value| value.offer.profile = CustodyProfile::ReleasedTerminal,
    ];
    for mutate in mutations {
        let mut changed = fixture.bootstrap.clone();
        mutate(&mut changed);
        assert!(validate_successor(&changed, &fixture.runtime, original).is_err());
    }
    let mut bootstrap = fixture.bootstrap.clone();
    bootstrap.request.lease_id = original.lease_id;
    assert!(validate_successor(&bootstrap, &fixture.runtime, original).is_err());
    bootstrap = fixture.bootstrap.clone();
    bootstrap.offer.offer_id = original.offer_id;
    assert!(validate_successor(&bootstrap, &fixture.runtime, original).is_err());
    let mut runtime = fixture.runtime.clone();
    runtime.controller_generation = fixture.request.controller_generation;
    assert!(validate_successor(&fixture.bootstrap, &runtime, original).is_err());
    let mut predecessor = original;
    predecessor.guardian = None;
    assert!(validate_successor(&fixture.bootstrap, &fixture.runtime, predecessor).is_err());
    predecessor = original;
    predecessor.holder = fixture.request.holder();
    assert!(validate_successor(&fixture.bootstrap, &fixture.runtime, predecessor).is_err());
    predecessor = original;
    predecessor.lease_id = CustodyLeaseId::new();
    assert!(validate_successor(&fixture.bootstrap, &fixture.runtime, predecessor).is_err());
}

#[test]
fn prepared_guardian_cannot_be_replaced_with_a_stale_generation_or_mixed_candidate() {
    let fixture = Fixture::new(ProcessIdentityRelation::Gone);
    let predecessor = fixture.snapshot.record().armed_lease().unwrap();
    let request = CustodyRequest::new(
        CustodyLeaseId::new(),
        fixture.request.runtime.clone(),
        predecessor.holder,
        fixture.request.auth_token().into(),
        fixture.request.controller_generation,
    );
    let mut offer = fixture.offer.clone();
    offer.lease_id = request.lease_id;
    let binding = CustodyGuardianBinding::new(
        Uuid::new_v4(),
        predecessor.guardian.unwrap().process(),
        &request,
        &offer,
    )
    .unwrap();
    let prepared = fixture
        .snapshot
        .record()
        .prepare_guardian(&request, &offer, binding)
        .unwrap();
    let mut bootstrap = fixture.bootstrap.clone();
    bootstrap.request.controller_generation += GENERATION_STEP;
    bootstrap.offer.offer_id = crate::pty_custody::CustodyOfferId::new();
    bootstrap.durable_predecessor =
        CustodyDurableSnapshot::new(prepared.clone(), fixture.snapshot.file_identity());
    validate_prepared_successor(&bootstrap, &fixture.runtime, &prepared).unwrap();
    let pending = CustodyDurableRecord::pending(prepared.runtime.clone());
    assert!(validate_prepared_successor(&bootstrap, &fixture.runtime, &pending).is_err());
    let mutations: &[fn(&mut crate::pty_custody::CustodyDurableLease)] = &[
        |value| value.guardian = None,
        |value| value.lease_id = CustodyLeaseId::new(),
    ];
    for mutate in mutations {
        let mut changed = prepared.clone();
        let CustodyDurableState::PreparedGuardian { candidate, .. } = &mut changed.state else {
            unreachable!()
        };
        mutate(candidate);
        assert!(validate_prepared_successor(&bootstrap, &fixture.runtime, &changed).is_err());
    }
    bootstrap.request.controller_generation = request.controller_generation;
    assert!(validate_prepared_successor(&bootstrap, &fixture.runtime, &prepared).is_err());
    bootstrap.request.controller_generation += GENERATION_STEP;
    bootstrap.request.lease_id = request.lease_id;
    assert!(validate_prepared_successor(&bootstrap, &fixture.runtime, &prepared).is_err());
    bootstrap.request.lease_id = CustodyLeaseId::new();
    bootstrap.offer.offer_id = offer.offer_id;
    assert!(validate_prepared_successor(&bootstrap, &fixture.runtime, &prepared).is_err());
}
