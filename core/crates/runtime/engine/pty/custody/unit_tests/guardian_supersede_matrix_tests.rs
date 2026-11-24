use super::*;
use crate::pty_custody::tests::{descriptor_set, digest, request};
use crate::pty_custody::{CustodyDescriptorSet, CustodyLeaseId, CustodyOfferId};
use uuid::Uuid;

const GENERATION_STEP: u64 = 1;
const PROCESS_STEP: u32 = 1;
const OTHER_DIGEST: u8 = 0x45;
const INVALID_VERSION: u16 = u16::MAX;

fn guardian(request: &CustodyRequest, offer: &CustodyOffer) -> CustodyGuardianBinding {
    CustodyGuardianBinding::new(Uuid::new_v4(), request.holder(), request, offer).unwrap()
}

#[test]
fn supersede_requires_new_holder_lease_offer_and_increasing_generation() {
    let original = request();
    let old_offer = CustodyOffer::new(&original, descriptor_set(), digest());
    let pending = CustodyDurableRecord::pending(original.runtime.clone());
    let prepared = pending
        .prepare_guardian(&original, &old_offer, guardian(&original, &old_offer))
        .unwrap();
    let mut successor = original.clone();
    successor.holder.pid += PROCESS_STEP;
    successor.lease_id = CustodyLeaseId::new();
    successor.controller_generation += GENERATION_STEP;
    let offer = CustodyOffer::new(&successor, descriptor_set(), digest());
    let valid = prepared
        .supersede_prepared_guardian(&successor, &offer, guardian(&successor, &offer))
        .unwrap();
    assert!(valid.is_valid());
    assert_eq!(
        valid.prepared_guardian_predecessor(),
        prepared.prepared_guardian_predecessor()
    );
    let mutations: &[fn(&mut CustodyRequest, &CustodyRequest)] = &[
        |value, old| value.holder = old.holder,
        |value, old| value.lease_id = old.lease_id,
        |value, old| value.controller_generation = old.controller_generation,
    ];
    for mutate in mutations {
        let mut changed = successor.clone();
        mutate(&mut changed, &original);
        let offer = CustodyOffer::new(&changed, descriptor_set(), digest());
        assert!(
            prepared
                .supersede_prepared_guardian(&changed, &offer, guardian(&changed, &offer))
                .is_none()
        );
    }
    let mut replay = offer.clone();
    replay.offer_id = old_offer.offer_id;
    assert!(
        prepared
            .supersede_prepared_guardian(&successor, &replay, guardian(&successor, &replay))
            .is_none()
    );
    assert!(
        pending
            .supersede_prepared_guardian(&successor, &offer, guardian(&successor, &offer))
            .is_none()
    );

    let mut invalid_record = prepared.clone();
    invalid_record.schema_version = INVALID_VERSION;
    assert!(
        invalid_record
            .supersede_prepared_guardian(&successor, &offer, guardian(&successor, &offer))
            .is_none()
    );
    let mut invalid_request = successor.clone();
    invalid_request.auth_token.clear();
    assert!(
        prepared
            .supersede_prepared_guardian(&invalid_request, &offer, guardian(&successor, &offer))
            .is_none()
    );
    let mut mismatch = offer.clone();
    mismatch.lease_id = CustodyLeaseId::new();
    assert!(
        prepared
            .supersede_prepared_guardian(&successor, &mismatch, guardian(&successor, &offer))
            .is_none()
    );
    mismatch = offer.clone();
    mismatch.offer_id = CustodyOfferId::new();
    assert!(
        prepared
            .supersede_prepared_guardian(&successor, &mismatch, guardian(&successor, &offer))
            .is_none()
    );
}

#[test]
fn supersede_preserves_descriptor_digest_runtime_and_monotonic_readonly_profile() {
    let initial = request();
    let old_offer = CustodyOffer::new(&initial, descriptor_set(), digest());
    let prepared = CustodyDurableRecord::pending(initial.runtime.clone())
        .prepare_guardian(&initial, &old_offer, guardian(&initial, &old_offer))
        .unwrap();
    let mut successor = initial.clone();
    successor.holder.pid += PROCESS_STEP;
    successor.lease_id = CustodyLeaseId::new();
    successor.controller_generation += GENERATION_STEP;
    for (descriptor, profile, digest, expected) in [
        (
            descriptor_set(),
            CustodyProfile::ActiveRunning,
            digest(),
            true,
        ),
        (
            descriptor_set(),
            CustodyProfile::ActiveTerminalReadOnly,
            digest(),
            true,
        ),
        (
            CustodyDescriptorSet::LinuxContainment,
            CustodyProfile::ActiveRunning,
            digest(),
            false,
        ),
        (
            descriptor_set(),
            CustodyProfile::ActiveRunning,
            crate::sha256_digest::Sha256Digest::repeated(OTHER_DIGEST),
            false,
        ),
        (
            CustodyDescriptorSet::ReleasedSuccessor,
            CustodyProfile::ReleasedTerminal,
            digest(),
            false,
        ),
    ] {
        let offer =
            CustodyOffer::new_with_profile(&successor, descriptor, profile, digest).unwrap();
        assert_eq!(
            prepared
                .supersede_prepared_guardian(&successor, &offer, guardian(&successor, &offer))
                .is_some(),
            expected
        );
    }
    successor.runtime.runtime_epoch = Uuid::new_v4();
    let offer = CustodyOffer::new(&successor, descriptor_set(), digest());
    assert!(
        prepared
            .supersede_prepared_guardian(&successor, &offer, guardian(&successor, &offer))
            .is_none()
    );
}
