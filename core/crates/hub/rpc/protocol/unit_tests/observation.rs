use crate::{
    commands::*,
    fixtures::{constants::*, *},
};
use yolo_hub_identity::RuntimeEpoch;
use yolo_hub_rpc_protocol::{Command, Method, Reply, Request, Response};
use yolo_hub_terminal_protocol::{LeaseObservation, PublicError, field};

#[test]
fn observing_a_writer_is_read_only_and_accepts_an_unheld_lease() {
    let command = Command::ObserveLease { target: target() };
    assert_eq!(command.method(), Method::ObserveLease);
    assert!(!command.mutates());
    assert_eq!(command.request_id(), None);
    assert_eq!(
        Request::new(hub(SECOND_HUB_ID), command.clone()),
        Err(PublicError::OriginMismatch)
    );
    let request = request(command);
    for lease in [None, Some(observation())] {
        let response = Response::new(&request, hub(HUB_ID), Reply::LeaseObservation { lease });
        assert_eq!(response.validate_for(&request), Ok(()));
    }
}

#[test]
fn observations_are_bound_to_origin_session_and_runtime() {
    let request = request(Command::ObserveLease { target: target() });
    let mut wrong_origin = observation().fields().clone();
    wrong_origin.target.address.hub_id = hub(SECOND_HUB_ID);
    let mut wrong_session = observation().fields().clone();
    wrong_session.target.address = address(HUB_ID, SECOND_SESSION_ID);
    let mut wrong_runtime = observation().fields().clone();
    wrong_runtime.target.runtime_epoch = RuntimeEpoch::from_uuid(id(SECOND_RUNTIME_ID)).unwrap();
    for fields in [wrong_origin, wrong_session, wrong_runtime] {
        let response = Response::new(
            &request,
            hub(HUB_ID),
            Reply::LeaseObservation {
                lease: Some(LeaseObservation::new(fields).unwrap()),
            },
        );
        assert_eq!(
            response.validate_for(&request),
            Err(PublicError::InvalidPayload)
        );
    }
}

#[test]
fn public_observation_wire_contains_no_write_capability_or_grant() {
    let observation = observation();
    let wire = serde_json::to_value(&observation).unwrap();
    assert!(wire.get(field::CAPABILITY).is_none());
    let mut forged = wire;
    forged[field::CAPABILITY] = serde_json::Value::String(CAPABILITY.into());
    assert!(serde_json::from_value::<LeaseObservation>(forged).is_err());
    let request = request(Command::ObserveLease { target: target() });
    let response = Response::new(&request, hub(HUB_ID), Reply::Lease { grant: grant() });
    assert_eq!(
        response.validate_for(&request),
        Err(PublicError::InvalidPayload)
    );
}
