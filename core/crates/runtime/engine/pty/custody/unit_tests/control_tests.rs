use super::*;

const SUCCESSOR_GENERATION: u64 = 10;
const STALE_CLOSURE_DIGEST_BYTE: u8 = 0x91;
const STALE_ATTESTATION_DIGEST_BYTE: u8 = 0x92;
const STALE_RECOVERY_DIGEST_BYTE: u8 = 0x93;

#[test]
fn release_and_ack_are_bound_to_the_exact_offer() {
    let exact_offer = offer();
    let control = CustodyControl::release(&exact_offer).unwrap();
    let acknowledgement = CustodyControlAck::released(&control).unwrap();

    assert!(control.matches_release(&exact_offer));
    assert_eq!(
        acknowledgement,
        CustodyControlAck::released(&control).unwrap()
    );
    assert!(!control.matches_release(&offer()));
    acknowledgement.into_exact_released(&control).unwrap();
}

#[test]
fn exact_released_ack_rejects_stale_lease_offer_and_closure_identity() {
    let control = CustodyControl::release(&offer()).unwrap();
    let CustodyControl::Release {
        lease_id,
        offer_id,
        closure_identity_sha256,
        settlement,
    } = &control
    else {
        unreachable!()
    };
    let stale = [
        CustodyControlAck::Released {
            lease_id: CustodyLeaseId::new(),
            offer_id: *offer_id,
            closure_identity_sha256: *closure_identity_sha256,
            settlement: settlement.clone(),
        },
        CustodyControlAck::Released {
            lease_id: *lease_id,
            offer_id: CustodyOfferId::new(),
            closure_identity_sha256: *closure_identity_sha256,
            settlement: settlement.clone(),
        },
        CustodyControlAck::Released {
            lease_id: *lease_id,
            offer_id: *offer_id,
            closure_identity_sha256: crate::sha256_digest::Sha256Digest::repeated(
                STALE_CLOSURE_DIGEST_BYTE,
            ),
            settlement: settlement.clone(),
        },
    ];
    for acknowledgement in stale {
        assert!(acknowledgement.into_exact_released(&control).is_err());
    }
}

#[test]
fn fallback_release_is_bound_to_runtime_offer_and_recovery_identity() {
    let exact_offer = offer();
    let settlement =
        CustodyReleaseSettlement::confirmed_bootstrap_global_exclusion(&exact_offer).unwrap();
    let control = CustodyControl::release_with_settlement(&exact_offer, settlement).unwrap();
    assert!(control.matches_release(&exact_offer));

    let mut stale_runtime = exact_offer.clone();
    stale_runtime.runtime.runtime_epoch = Uuid::new_v4();
    let mut stale_recovery = exact_offer.clone();
    stale_recovery.recovery_identity_sha256 =
        crate::sha256_digest::Sha256Digest::repeated(STALE_RECOVERY_DIGEST_BYTE);
    let mut stale_offer = exact_offer.clone();
    stale_offer.offer_id = CustodyOfferId::new();
    let mut stale_lease = exact_offer.clone();
    stale_lease.lease_id = CustodyLeaseId::new();
    for replay in [stale_runtime, stale_recovery, stale_offer, stale_lease] {
        assert!(!control.matches_release(&replay));
    }
}

#[test]
fn fallback_release_rejects_forged_attestation_and_non_macos_profile() {
    let exact_offer = offer();
    let forged = CustodyReleaseSettlement::ConfirmedBootstrapGlobalExclusion {
        attestation_sha256: crate::sha256_digest::Sha256Digest::repeated(
            STALE_ATTESTATION_DIGEST_BYTE,
        ),
    };
    assert!(CustodyControl::release_with_settlement(&exact_offer, forged).is_err());

    let mut released_successor = exact_offer;
    released_successor.descriptor_set = CustodyDescriptorSet::ReleasedSuccessor;
    assert!(
        CustodyReleaseSettlement::confirmed_bootstrap_global_exclusion(&released_successor)
            .is_err()
    );
}

#[test]
fn released_ack_rejects_settlement_mode_substitution() {
    let exact_offer = offer();
    let settlement =
        CustodyReleaseSettlement::confirmed_bootstrap_global_exclusion(&exact_offer).unwrap();
    let control = CustodyControl::release_with_settlement(&exact_offer, settlement).unwrap();
    let CustodyControl::Release {
        lease_id,
        offer_id,
        closure_identity_sha256,
        ..
    } = &control
    else {
        unreachable!()
    };
    let substituted = CustodyControlAck::Released {
        lease_id: *lease_id,
        offer_id: *offer_id,
        closure_identity_sha256: *closure_identity_sha256,
        settlement: CustodyReleaseSettlement::exact(),
    };
    assert!(substituted.into_exact_released(&control).is_err());
}

#[test]
fn successor_offer_mints_a_fresh_fallback_attestation() {
    let predecessor = offer();
    let successor = CustodyOffer::new(
        &super::tests::request(),
        super::tests::descriptor_set(),
        predecessor.recovery_identity_sha256,
    );
    let settlement =
        CustodyReleaseSettlement::confirmed_bootstrap_global_exclusion(&predecessor).unwrap();
    let control = CustodyControl::release_with_settlement(&predecessor, settlement).unwrap();
    let successor_settlement =
        CustodyReleaseSettlement::confirmed_bootstrap_global_exclusion(&successor).unwrap();

    assert!(!control.matches_release(&successor));
    assert_ne!(control.release_settlement(), Some(&successor_settlement));
}

#[test]
fn supersede_and_ack_bind_both_offers_and_the_generation() {
    let predecessor = offer();
    let successor = offer();
    let control = CustodyControl::supersede(&predecessor, &successor, SUCCESSOR_GENERATION);
    let acknowledgement = CustodyControlAck::superseded(&control).unwrap();

    assert_eq!(
        acknowledgement,
        CustodyControlAck::superseded(&control).unwrap()
    );
    assert_ne!(
        acknowledgement,
        CustodyControlAck::superseded(&CustodyControl::supersede(
            &predecessor,
            &offer(),
            SUCCESSOR_GENERATION,
        ))
        .unwrap()
    );
}

fn offer() -> CustodyOffer {
    CustodyOffer::new(
        &super::tests::request(),
        super::tests::descriptor_set(),
        super::tests::digest(),
    )
}
