use super::*;
use crate::protocol::field;
use crate::pty_custody::protocol::constants::{
    ACTIVE_RUNNING_WIRE_VALUE, ACTIVE_TERMINAL_READ_ONLY_WIRE_VALUE, PROFILE_FIELD,
    RELEASED_TERMINAL_WIRE_VALUE, UNKNOWN_PROFILE_WIRE_VALUE,
};
use crate::pty_custody::tests::{descriptor_set, digest, request};
use crate::pty_custody::{CustodyDescriptorSet, CustodyProfile};
use crate::pty_custody::{CustodyOffer, PROTOCOL_VERSION, arm_fixture};

const NEXT_VERSION_OFFSET: u16 = 1;

#[test]
fn durable_records_require_the_current_schema_in_both_directions() {
    let pending = CustodyDurableRecord::pending(request().runtime);
    let encoded = serde_json::to_value(&pending).unwrap();
    let accepted: CustodyDurableRecord = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(accepted, pending);
    for version in (u16::default()..CUSTODY_RECORD_SCHEMA_VERSION)
        .chain([CUSTODY_RECORD_SCHEMA_VERSION + NEXT_VERSION_OFFSET])
    {
        let mut changed = pending.clone();
        changed.schema_version = version;
        assert!(!changed.is_valid());
        assert!(serde_json::to_value(&changed).is_err());
        let mut incoming = encoded.clone();
        incoming[field::SCHEMA_VERSION] = version.into();
        assert!(serde_json::from_value::<CustodyDurableRecord>(incoming).is_err());
    }
}

#[test]
fn older_and_future_protocols_cannot_create_or_match_an_offer() {
    for version in
        (u16::default()..PROTOCOL_VERSION).chain([PROTOCOL_VERSION + NEXT_VERSION_OFFSET])
    {
        let mut changed = request();
        let mut offer = CustodyOffer::new(&changed, descriptor_set(), digest());
        changed.protocol_version = version;
        offer.protocol_version = version;
        assert!(!changed.is_well_formed());
        assert!(!offer.matches_request(&changed));
        assert!(
            CustodyDurableRecord::pending(changed.runtime.clone())
                .arm(&changed, &offer)
                .is_none()
        );
    }
}

#[test]
fn armed_records_require_their_explicit_descriptor_attestation() {
    let request = request();
    let offer = CustodyOffer::new(&request, descriptor_set(), digest());
    let armed = arm_fixture(
        &CustodyDurableRecord::pending(request.runtime.clone()),
        &request,
        &offer,
    )
    .unwrap();
    let mut incoming = serde_json::to_value(&armed).unwrap();
    incoming[field::STATE]
        .as_object_mut()
        .unwrap()
        .remove(field::DESCRIPTOR_SET);
    assert!(serde_json::from_value::<CustodyDurableRecord>(incoming).is_err());
}

#[test]
fn every_offer_and_durable_lease_requires_its_explicit_profile() {
    for (profile, wire_value) in [
        (CustodyProfile::ActiveRunning, ACTIVE_RUNNING_WIRE_VALUE),
        (
            CustodyProfile::ActiveTerminalReadOnly,
            ACTIVE_TERMINAL_READ_ONLY_WIRE_VALUE,
        ),
        (
            CustodyProfile::ReleasedTerminal,
            RELEASED_TERMINAL_WIRE_VALUE,
        ),
    ] {
        let request = request();
        let descriptors = if profile == CustodyProfile::ReleasedTerminal {
            CustodyDescriptorSet::ReleasedSuccessor
        } else {
            descriptor_set()
        };
        let offer =
            CustodyOffer::new_with_profile(&request, descriptors, profile, digest()).unwrap();
        let armed = arm_fixture(
            &CustodyDurableRecord::pending(request.runtime.clone()),
            &request,
            &offer,
        )
        .unwrap();
        let lease = armed.armed_lease().unwrap();
        let retired = armed.retire(lease).unwrap();

        assert_profile_contract(&offer, wire_value, false);
        assert_profile_contract(&lease, wire_value, false);
        assert_profile_contract(&armed, wire_value, true);
        assert_profile_contract(&retired, wire_value, true);
    }
}

fn assert_profile_contract<T>(expected: &T, wire_value: &str, nested_state: bool)
where
    T: Serialize + serde::de::DeserializeOwned + std::fmt::Debug + PartialEq,
{
    let value = serde_json::to_value(expected).unwrap();
    let profile = if nested_state {
        &value[field::STATE]
    } else {
        &value
    };
    assert_eq!(profile[PROFILE_FIELD], wire_value);
    assert_eq!(
        &serde_json::from_value::<T>(value.clone()).unwrap(),
        expected
    );
    let frame = crate::protocol::encode(expected).unwrap();
    assert_eq!(
        &crate::protocol::decode::<T>(&frame[crate::protocol::FRAME_LENGTH_BYTES..]).unwrap(),
        expected,
    );
    for replacement in [
        None,
        Some(serde_json::Value::Null),
        Some(UNKNOWN_PROFILE_WIRE_VALUE.into()),
    ] {
        let mut invalid = value.clone();
        let object = if nested_state {
            &mut invalid[field::STATE]
        } else {
            &mut invalid
        };
        let object = object.as_object_mut().unwrap();
        match replacement {
            Some(value) => {
                object.insert(PROFILE_FIELD.to_owned(), value);
            }
            None => {
                object.remove(PROFILE_FIELD);
            }
        }
        assert!(serde_json::from_value::<T>(invalid.clone()).is_err());
        let frame = crate::protocol::encode(&invalid).unwrap();
        assert!(
            crate::protocol::decode::<T>(&frame[crate::protocol::FRAME_LENGTH_BYTES..]).is_err()
        );
    }
}
