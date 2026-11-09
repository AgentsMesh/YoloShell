use crate::fixtures::{self, constants::*, reject_field, roundtrip};
use yolo_hub_terminal_protocol::*;

fn fields() -> LeaseObservationFields {
    LeaseObservationFields {
        target: fixtures::target(),
        generation: FIRST_LEASE_GENERATION,
        holder_id: NAME.into(),
        controller_instance_id: Some(NAME.into()),
        expires_at_ms: LEASE_EXPIRY_MS,
    }
}

#[test]
fn observation_is_read_only_and_preserves_non_uuid_legacy_holders() {
    let observed = LeaseObservation::new(fields()).unwrap();
    assert_eq!(observed.fields().holder_id, NAME);
    roundtrip(&observed);
    reject_field(&observed, UNKNOWN_FIELD, UNKNOWN_VALUE.into());
    let mut raw: LeaseObservationFields = observed.into();
    raw.controller_instance_id = None;
    roundtrip(&LeaseObservation::new(raw).unwrap());
}

#[test]
fn observations_reject_invalid_generations_expiry_and_holder_metadata() {
    let mutations: &[fn(&mut LeaseObservationFields)] = &[
        |f| f.generation = 0,
        |f| f.expires_at_ms = 0,
        |f| f.holder_id.clear(),
        |f| f.holder_id = NAME.repeat(MAX_WRITER_ID_BYTES),
        |f| f.controller_instance_id = Some(INVALID_TEXT[0].into()),
        |f| f.controller_instance_id = Some(NAME.repeat(MAX_WRITER_ID_BYTES)),
    ];
    for mutate in mutations {
        let mut raw = fields();
        mutate(&mut raw);
        assert!(LeaseObservation::new(raw).is_err());
    }
}

#[test]
fn takeover_requires_callers_nonzero_observed_generation_on_both_paths() {
    let mode = LeaseMode::TakeControl {
        observed_generation: 0,
    };
    assert_eq!(mode.validate(), Err(PublicError::InvalidLease));
    assert!(serde_json::from_value::<LeaseMode>(serde_json::to_value(mode).unwrap()).is_err());
    let request = LeaseRequest {
        request_id: uuid::Uuid::new_v4(),
        target: fixtures::target(),
        principal: fixtures::principal(),
        mode,
    };
    assert_eq!(request.validate(), Err(PublicError::InvalidLease));
    let mode = LeaseMode::TakeControl {
        observed_generation: FIRST_LEASE_GENERATION + STEP,
    };
    assert_eq!(mode.validate(), Ok(()));
    roundtrip(&mode);
}
