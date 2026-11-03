use crate::{fixtures::*, history_support::*, support::*};
use yolo_hub_delegation::{FIRST_POLICY_EPOCH, PeerIdentity};
use yolo_hub_federation_directory::ObservationHistory;
use yolo_hub_federation_settings::routing_limits;
use yolo_hub_identity::HubId;
use yolo_hub_peer_protocol::{FIRST_REVISION, MAX_CATALOGUE_LIFETIME_MS, Purpose, Signed};

#[test]
fn history_constructor_preserves_entry_and_exposes_canonical_exact_size_catalogues() {
    let entry = Identity::new();
    let (first, second) = ordered_origins();
    let expected = vec![catalogue(&first), catalogue(&second)];
    let reversed = expected.iter().rev().cloned().collect();
    let history = history(entry.identity, reversed);
    assert_eq!(history.entry(), entry.identity);
    let mut catalogues = history.catalogues();
    assert_eq!(catalogues.len(), expected.len());
    assert_eq!(catalogues.next(), expected.first());
    assert_eq!(catalogues.len(), SINGLE_ORIGIN);
    assert_eq!(catalogues.next(), expected.last());
    assert!(catalogues.next().is_none());
    history.validate(routing_limits()).unwrap();
    let empty = ObservationHistory::new(entry.identity, Vec::new(), routing_limits()).unwrap();
    assert!(empty.catalogues().next().is_none());
}

#[test]
fn history_constructor_rejects_tampered_signatures_wrong_purposes_and_signers() {
    let fixture = Fixture::new();
    let valid = fixture.export.catalogue;
    let mut tampered = valid.clone();
    tampered.payload.terminals.clear();
    let mut truncated = valid.clone();
    truncated.signature.pop();
    let mut wrong_signer = valid.clone();
    wrong_signer.signer = fixture.relay.identity;
    let wrong_purpose = Signed::new(valid.payload, Purpose::Call, &fixture.origin).unwrap();
    for candidate in [tampered, truncated, wrong_signer, wrong_purpose] {
        assert!(
            ObservationHistory::new(fixture.entry.identity, vec![candidate], routing_limits())
                .is_err()
        );
    }
}

#[test]
fn valid_signatures_do_not_bypass_catalogue_structure_validation() {
    let fixture = Fixture::new();
    let valid = fixture.export.catalogue.payload;
    let mut zero_revision = valid.clone();
    zero_revision.revision = FIRST_REVISION - STEP;
    let mut zero_policy = valid.clone();
    zero_policy.policy_epoch = FIRST_POLICY_EPOCH - STEP;
    let mut invalid_lifetime = valid.clone();
    invalid_lifetime.expires_at_ms = invalid_lifetime.issued_at_ms;
    let mut excessive_lifetime = valid.clone();
    excessive_lifetime.expires_at_ms =
        excessive_lifetime.issued_at_ms + MAX_CATALOGUE_LIFETIME_MS + STEP;
    let mut foreign_session = valid.clone();
    foreign_session.terminals = vec![session(fixture.relay.identity)];
    let mut duplicate_session = valid;
    duplicate_session
        .terminals
        .extend(duplicate_session.terminals.clone());
    for payload in [
        zero_revision,
        zero_policy,
        invalid_lifetime,
        excessive_lifetime,
        foreign_session,
        duplicate_session,
    ] {
        let signed = resign(payload, &fixture.origin);
        assert!(
            ObservationHistory::new(fixture.entry.identity, vec![signed], routing_limits())
                .is_err()
        );
    }
}

#[test]
fn history_constructor_rejects_local_hub_and_local_key_aliases() {
    let mut local = Identity::new();
    let entry = local.identity;
    let local_catalogue = catalogue(&local);
    let local_hub_other_key = catalogue(&rotate(entry));
    local.identity = PeerIdentity::new(HubId::new(), *entry.key()).unwrap();
    let local_key_other_hub = catalogue(&local);
    for candidate in [local_catalogue, local_hub_other_key, local_key_other_hub] {
        assert!(ObservationHistory::new(entry, vec![candidate], routing_limits()).is_err());
    }
}

#[test]
fn history_constructor_rejects_duplicate_origins_even_when_observations_match() {
    let entry = Identity::new();
    let origin = Identity::new();
    let first = catalogue(&origin);
    let mut later = first.payload.clone();
    later.revision += STEP;
    for duplicate in [
        first.clone(),
        resign(later, &origin),
        catalogue(&rotate(origin.identity)),
    ] {
        assert!(
            ObservationHistory::new(
                entry.identity,
                vec![first.clone(), duplicate],
                routing_limits()
            )
            .is_err()
        );
    }
}

#[test]
fn history_constructor_and_revalidation_enforce_the_supplied_origin_capacity() {
    let entry = Identity::new();
    let (first, second) = ordered_origins();
    let catalogues = vec![catalogue(&first), catalogue(&second)];
    let limits = origin_limit(SINGLE_ORIGIN);
    assert!(ObservationHistory::new(entry.identity, catalogues.clone(), limits).is_err());
    let history = history(entry.identity, catalogues);
    assert!(history.validate(limits).is_err());
    assert!(ObservationHistory::new(entry.identity, vec![catalogue(&first)], limits).is_ok());
}
