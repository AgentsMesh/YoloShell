use super::protocol::constants::{
    ACTIVE_RUNNING_WIRE_VALUE, ACTIVE_TERMINAL_READ_ONLY_WIRE_VALUE, PROFILE_FIELD,
    UNKNOWN_PROFILE_WIRE_VALUE,
};
use super::tests::{descriptor_set, digest, request};
use super::*;

#[test]
fn active_running_offer_requires_an_explicit_profile() {
    let offer = CustodyOffer::new(&request(), descriptor_set(), digest());
    let value = serde_json::to_value(&offer).unwrap();

    assert_eq!(value[PROFILE_FIELD], ACTIVE_RUNNING_WIRE_VALUE);
    let decoded: CustodyOffer = serde_json::from_value(value).unwrap();
    assert_eq!(decoded.profile, CustodyProfile::ActiveRunning);
    assert_eq!(decoded, offer);
}

#[test]
fn terminal_read_only_profile_round_trips_as_a_bounded_marker() {
    let offer = CustodyOffer::new_with_profile(
        &request(),
        descriptor_set(),
        CustodyProfile::ActiveTerminalReadOnly,
        digest(),
    )
    .unwrap();
    let value = serde_json::to_value(&offer).unwrap();

    assert_eq!(
        value.get(PROFILE_FIELD).and_then(serde_json::Value::as_str),
        Some(ACTIVE_TERMINAL_READ_ONLY_WIRE_VALUE)
    );
    assert_eq!(
        serde_json::from_value::<CustodyOffer>(value).unwrap(),
        offer
    );
}

#[test]
fn profile_rejects_unknown_values_and_descriptor_mismatches() {
    let mut value =
        serde_json::to_value(CustodyOffer::new(&request(), descriptor_set(), digest())).unwrap();
    value.as_object_mut().unwrap().insert(
        PROFILE_FIELD.to_owned(),
        serde_json::Value::String(UNKNOWN_PROFILE_WIRE_VALUE.to_owned()),
    );

    assert!(serde_json::from_value::<CustodyOffer>(value).is_err());
    assert!(
        CustodyOffer::new_with_profile(
            &request(),
            CustodyDescriptorSet::ReleasedSuccessor,
            CustodyProfile::ActiveTerminalReadOnly,
            digest(),
        )
        .is_err()
    );
    assert!(
        CustodyOffer::new_with_profile(
            &request(),
            descriptor_set(),
            CustodyProfile::ReleasedTerminal,
            digest(),
        )
        .is_err()
    );
}

#[test]
fn durable_candidate_binds_terminal_read_only_profile() {
    let request = request();
    let offer = CustodyOffer::new_with_profile(
        &request,
        descriptor_set(),
        CustodyProfile::ActiveTerminalReadOnly,
        digest(),
    )
    .unwrap();
    let pending = CustodyDurableRecord::pending(request.runtime.clone());
    let armed = crate::pty_custody::arm_fixture(&pending, &request, &offer).unwrap();
    let lease = armed.armed_lease().unwrap();

    assert_eq!(lease.profile, CustodyProfile::ActiveTerminalReadOnly);
    assert!(armed.matches_candidate(&offer, lease));
    let mut changed = offer;
    changed.profile = CustodyProfile::ActiveRunning;
    assert!(!armed.matches_candidate(&changed, lease));
}

#[test]
fn released_descriptor_constructor_uses_released_terminal_profile() {
    let offer = CustodyOffer::new(
        &request(),
        CustodyDescriptorSet::ReleasedSuccessor,
        digest(),
    );

    assert_eq!(offer.profile, CustodyProfile::ReleasedTerminal);
    assert!(offer.matches_request(&request_with_runtime(&offer)));
}

fn request_with_runtime(offer: &CustodyOffer) -> CustodyRequest {
    let mut exact = request();
    exact.lease_id = offer.lease_id;
    exact.runtime = offer.runtime.clone();
    exact
}
