use super::tests::{descriptor_set, digest, request};
use super::*;
use crate::paths::identity::FileIdentity;
use uuid::Uuid;

const SUCCESSOR_GENERATION_INCREMENT: u64 = 1;
const LAST_OUTPUT_SEQUENCE: u64 = 1;
const IDENTITY_MAX_BYTES: u64 = u64::MAX;
const INCOMPATIBLE_PROTOCOL_VERSION_OFFSET: u16 = 1;
const UNSUPPORTED_PROTOCOL_VERSION: u16 =
    PROTOCOL_VERSION.saturating_add(INCOMPATIBLE_PROTOCOL_VERSION_OFFSET);

#[test]
fn durable_schemas_reject_wire_protocol_fields() {
    let request = request();
    let offer = CustodyOffer::new(&request, descriptor_set(), digest());
    let pending = CustodyDurableRecord::pending(request.runtime.clone());
    let guardian =
        CustodyGuardianBinding::new(Uuid::new_v4(), request.holder, &request, &offer).unwrap();

    assert_durable_protocol_isolation(pending);
    assert_durable_protocol_isolation(guardian);
}

#[test]
fn wire_schemas_reject_unsupported_protocol_versions() {
    let mut unsupported_request = request();
    unsupported_request.protocol_version = UNSUPPORTED_PROTOCOL_VERSION;
    assert!(!unsupported_request.is_well_formed());

    let request = request();
    let mut unsupported_offer = CustodyOffer::new(&request, descriptor_set(), digest());
    unsupported_offer.protocol_version = UNSUPPORTED_PROTOCOL_VERSION;
    assert!(!unsupported_offer.matches_request(&request));
}

#[test]
fn durable_record_rearms_only_for_a_higher_controller_generation() {
    let first_request = request();
    let pending = CustodyDurableRecord::pending(first_request.runtime.clone());
    let first_offer = CustodyOffer::new(&first_request, descriptor_set(), digest());
    let armed = arm_fixture(&pending, &first_request, &first_offer).unwrap();
    assert!(armed.rearm(&first_request, &first_offer).is_none());

    let mut successor = request();
    successor.runtime = armed.runtime.clone();
    successor.controller_generation = first_request
        .controller_generation
        .saturating_add(SUCCESSOR_GENERATION_INCREMENT);
    let successor_offer = CustodyOffer::new(&successor, descriptor_set(), digest());
    assert!(rearm_fixture(&armed, &successor, &successor_offer).is_some());
}

#[test]
fn exact_retired_lease_rearms_as_a_released_successor() {
    let first_request = request();
    let first_offer = CustodyOffer::new(&first_request, descriptor_set(), digest());
    let pending = CustodyDurableRecord::pending(first_request.runtime.clone());
    let armed = arm_fixture(&pending, &first_request, &first_offer).unwrap();
    let exact = armed.armed_lease().unwrap();
    let retired = armed.retire(exact).unwrap();
    assert!(retired.armed_lease().is_none());
    assert_eq!(retired.retired_lease(), Some(exact));

    let mut successor = request();
    successor.runtime = retired.runtime.clone();
    successor.controller_generation = exact
        .controller_generation
        .saturating_add(SUCCESSOR_GENERATION_INCREMENT);
    let successor_offer = CustodyOffer::new(
        &successor,
        CustodyDescriptorSet::ReleasedSuccessor,
        digest(),
    );
    assert!(retired.rearm(&successor, &successor_offer).is_some());
}

#[test]
fn retirement_rejects_a_changed_exact_lease() {
    let request = request();
    let offer = CustodyOffer::new(&request, descriptor_set(), digest());
    let pending = CustodyDurableRecord::pending(request.runtime.clone());
    let armed = arm_fixture(&pending, &request, &offer).unwrap();
    let mut changed = armed.armed_lease().unwrap();
    changed.controller_generation = changed
        .controller_generation
        .saturating_add(SUCCESSOR_GENERATION_INCREMENT);

    assert!(armed.retire(changed).is_none());
    assert!(armed.retired_lease().is_none());
}

#[test]
fn prepared_guardian_retires_only_with_its_exact_candidate_and_evidence() {
    let request = request();
    let offer = CustodyOffer::new(&request, descriptor_set(), digest());
    let guardian =
        CustodyGuardianBinding::new(Uuid::new_v4(), request.holder, &request, &offer).unwrap();
    let prepared = CustodyDurableRecord::pending(request.runtime.clone())
        .prepare_guardian(&request, &offer, guardian)
        .unwrap();
    let candidate = prepared.prepared_guardian_candidate().unwrap();
    let identity = file_identity();
    let evidence = CustodyRetirementEvidence::new(
        request.controller_generation,
        identity,
        identity,
        identity,
        LAST_OUTPUT_SEQUENCE,
    );
    let retired = prepared
        .retire_prepared_guardian_with_evidence(candidate, evidence)
        .unwrap();

    assert_eq!(retired.retired_lease(), Some(candidate));
    assert_eq!(retired.retirement_evidence(), Some(evidence));
    let encoded = serde_json::to_value(&retired).unwrap();
    let decoded: CustodyDurableRecord = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded, retired);
    let mut changed = candidate;
    changed.lease_id = CustodyLeaseId::new();
    assert!(
        prepared
            .retire_prepared_guardian_with_evidence(changed, evidence)
            .is_none()
    );
}

fn file_identity() -> FileIdentity {
    let file = tempfile::NamedTempFile::new().unwrap();
    FileIdentity::inspect_bounded(
        &file.path().metadata().unwrap(),
        file.path(),
        IDENTITY_MAX_BYTES,
    )
    .unwrap()
}

fn assert_durable_protocol_isolation<T>(value: T)
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let mut encoded = serde_json::to_value(value).unwrap();
    let object = encoded.as_object_mut().unwrap();
    assert!(!object.contains_key(crate::protocol::field::PROTOCOL_VERSION));
    object.insert(
        crate::protocol::field::PROTOCOL_VERSION.to_owned(),
        serde_json::to_value(PROTOCOL_VERSION).unwrap(),
    );
    assert!(serde_json::from_value::<T>(encoded).is_err());
}
