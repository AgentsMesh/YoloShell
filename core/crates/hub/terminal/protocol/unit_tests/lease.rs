use crate::fixtures::{self, constants::*, roundtrip};
use uuid::Uuid;
use yolo_hub_terminal_protocol::*;

#[test]
fn writer_identity_is_client_neutral_and_lease_debug_is_redacted() {
    let grant = LeaseGrant::new(fixtures::grant_fields()).unwrap();
    assert_eq!(grant.fields().principal, fixtures::principal());
    roundtrip(&grant);
    let debug = format!("{grant:?}");
    assert!(debug.contains(REDACTED));
    assert!(!debug.contains(CAPABILITY_ID));
    let raw: LeaseGrantFields = grant.into();
    assert_eq!(raw.target, fixtures::target());
}

#[test]
fn lease_grant_checks_every_time_and_identity_bound() {
    let mutations: &[fn(&mut LeaseGrantFields)] = &[
        |f| f.principal.client_id = Uuid::nil(),
        |f| f.principal.controller_instance_id = Uuid::nil(),
        |f| f.generation = 0,
        |f| f.expires_at_ms = 0,
        |f| f.expires_at_ms = -LEASE_EXPIRY_MS,
        |f| f.ttl_ms = 0,
        |f| f.ttl_ms = MAX_LEASE_TTL_MS + STEP,
        |f| f.renew_after_ms = 0,
        |f| f.renew_after_ms = f.ttl_ms,
        |f| f.renew_after_ms = f.ttl_ms + STEP,
    ];
    for mutate in mutations {
        let mut fields = fixtures::grant_fields();
        mutate(&mut fields);
        assert!(LeaseGrant::new(fields).is_err());
    }
    let mut fields = fixtures::grant_fields();
    fields.ttl_ms = MAX_LEASE_TTL_MS;
    roundtrip(&LeaseGrant::new(fields).unwrap());
}

#[test]
fn acquisition_and_takeover_preserve_controller_identity() {
    for mode in [
        LeaseMode::Acquire {},
        LeaseMode::TakeControl {
            observed_generation: FIRST_LEASE_GENERATION,
        },
    ] {
        let mut request = LeaseRequest {
            request_id: Uuid::from_u128(REQUEST_ID),
            target: fixtures::target(),
            principal: fixtures::principal(),
            mode,
        };
        assert_eq!(request.validate(), Ok(()));
        roundtrip(&request);
        request.request_id = Uuid::nil();
        assert_eq!(request.validate(), Err(PublicError::InvalidRequest));
        request.request_id = Uuid::from_u128(REQUEST_ID);
        request.principal.client_id = Uuid::nil();
        assert_eq!(request.validate(), Err(PublicError::InvalidRequest));
    }
}
