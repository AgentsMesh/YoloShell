use super::super::constants::code;
use super::super::{LeaseAuthority, LeaseEventPayload, LeaseReason, LeaseRequest, LeaseTransition};
use super::request;
use yolo_terminal_model::WriterCapability;
use yolo_wire::field;

#[test]
fn public_snapshots_and_events_never_expose_capabilities() {
    let mut authority = LeaseAuthority::default();
    let mut transitions: Vec<LeaseTransition> = Vec::new();
    let owner = request("desktop", "main");
    let (grant, snapshot, revoked, expired) = {
        let mut emit = |transition| transitions.push(transition);
        let grant = authority.acquire(&owner, false, &mut emit).unwrap();
        let snapshot = authority.snapshot(&mut emit).unwrap();
        let revoked = authority
            .revoke(LeaseReason::SessionEnded, &mut emit)
            .unwrap();
        authority.acquire(&owner, false, &mut emit).unwrap();
        authority.expire_for_test();
        let expired = authority.expire(&mut emit).unwrap();
        (grant, snapshot, revoked, expired)
    };
    let snapshot_json = serde_json::to_value(snapshot).unwrap();
    assert!(snapshot_json.get(field::WRITER_CAPABILITY).is_none());
    for lease in [revoked, expired] {
        let lease_json = serde_json::to_value(lease).unwrap();
        assert!(lease_json.get(field::WRITER_CAPABILITY).is_none());
    }
    for transition in transitions {
        let event_json = serde_json::to_value(LeaseEventPayload::from(transition)).unwrap();
        assert!(event_json.get(field::WRITER_CAPABILITY).is_none());
    }
    assert!(serde_json::to_value(grant).unwrap()[field::WRITER_CAPABILITY].is_string());
}

#[test]
fn correct_generation_with_wrong_capability_is_rejected() {
    let mut authority = LeaseAuthority::default();
    let mut emit = |_| {};
    let grant = authority
        .acquire(&request("desktop", "main"), false, &mut emit)
        .unwrap();
    let wrong = WriterCapability::generate();
    let wrong_wire = serde_json::to_string(&wrong).unwrap();
    let error = authority
        .validate_authorization(grant.generation, &wrong, &mut emit)
        .unwrap_err();
    assert_eq!(error.code, code::NOT_HOLDER);
    assert!(!format!("{error:?}").contains(field::WRITER_CAPABILITY));
    assert!(
        !serde_json::to_string(&error)
            .unwrap()
            .contains(wrong_wire.trim_matches('"'))
    );
}

#[test]
fn same_identity_cannot_recover_with_a_different_capability() {
    let mut authority = LeaseAuthority::default();
    let mut emit = |_| {};
    let original = request("desktop", "main");
    let grant = authority.acquire(&original, false, &mut emit).unwrap();
    let secret = serde_json::to_string(&original.writer_capability).unwrap();
    assert!(!format!("{authority:?}").contains(secret.trim_matches('"')));
    let different = request("desktop", "main");
    let different_secret = serde_json::to_string(&different.writer_capability).unwrap();
    let error = authority.acquire(&different, false, &mut emit).unwrap_err();
    assert_eq!(error.code, code::NOT_HOLDER);
    let error_wire = serde_json::to_string(&error).unwrap();
    assert!(!error_wire.contains(secret.trim_matches('"')));
    assert!(!error_wire.contains(different_secret.trim_matches('"')));
    let current = authority.snapshot(&mut emit).unwrap();
    assert_eq!(current.generation, grant.generation);
}

#[test]
fn renew_and_release_require_the_bound_capability_and_instance() {
    let mut authority = LeaseAuthority::default();
    let mut emit = |_| {};
    let owner = request("desktop", "main");
    let grant = authority.acquire(&owner, false, &mut emit).unwrap();
    let mut unauthorized = owner.clone();
    unauthorized.generation = Some(grant.generation);
    unauthorized.writer_capability = WriterCapability::generate();
    assert_eq!(
        authority.renew(&unauthorized, &mut emit).unwrap_err().code,
        code::NOT_HOLDER
    );
    assert_eq!(
        authority
            .release(&unauthorized, &mut emit)
            .unwrap_err()
            .code,
        code::NOT_HOLDER
    );
    let mut missing_instance = owner.clone();
    missing_instance.generation = Some(grant.generation);
    missing_instance.controller_instance_id = None;
    assert_eq!(
        authority
            .renew(&missing_instance, &mut emit)
            .unwrap_err()
            .code,
        code::CONTROLLER_INSTANCE_MISMATCH
    );
    assert_eq!(
        authority
            .release(&missing_instance, &mut emit)
            .unwrap_err()
            .code,
        code::CONTROLLER_INSTANCE_MISMATCH
    );
    let mut authorized = owner;
    authorized.generation = Some(grant.generation);
    let renewed = authority.renew(&authorized, &mut emit).unwrap();
    assert_eq!(renewed.writer_capability, authorized.writer_capability);
    authority.release(&authorized, &mut emit).unwrap();
}

#[test]
fn takeover_requires_generation_and_rebinds_capability() {
    let mut authority = LeaseAuthority::default();
    let mut emit = |_| {};
    let first = authority
        .acquire(&request("first", "main"), false, &mut emit)
        .unwrap();
    let mut takeover = request("second", "main");
    assert_eq!(
        authority
            .acquire(&takeover, true, &mut emit)
            .unwrap_err()
            .code,
        code::STALE_GENERATION
    );
    takeover.generation = Some(first.generation);
    let replacement = authority.acquire(&takeover, true, &mut emit).unwrap();
    assert_eq!(replacement.writer_capability, takeover.writer_capability);
    assert_eq!(
        authority
            .validate_authorization(replacement.generation, &first.writer_capability, &mut emit,)
            .unwrap_err()
            .code,
        code::NOT_HOLDER
    );
    authority
        .validate_authorization(
            replacement.generation,
            &replacement.writer_capability,
            &mut emit,
        )
        .unwrap();
}

#[test]
fn same_public_identity_requires_cas_to_rebind_a_lost_capability() {
    let mut authority = LeaseAuthority::default();
    let mut emit = |_| {};
    let original = request("desktop", "main");
    let first = authority.acquire(&original, false, &mut emit).unwrap();
    let mut rebuilt = request("desktop", "main");
    assert_eq!(
        authority
            .acquire(&rebuilt, false, &mut emit)
            .unwrap_err()
            .code,
        code::NOT_HOLDER
    );
    rebuilt.generation = Some(first.generation);
    let replacement = authority.acquire(&rebuilt, true, &mut emit).unwrap();
    assert!(replacement.generation > first.generation);
    assert_eq!(replacement.writer_capability, rebuilt.writer_capability);
}

#[test]
fn lease_request_requires_the_exact_capability_field() {
    let value = serde_json::to_value(request("desktop", "main")).unwrap();
    let mut missing = value.as_object().unwrap().clone();
    let capability = missing.remove(field::WRITER_CAPABILITY).unwrap();
    assert!(
        serde_json::from_value::<LeaseRequest>(serde_json::Value::Object(missing.clone())).is_err()
    );
    missing.insert(field::UNSUPPORTED_WRITER_TOKEN.into(), capability);
    assert!(serde_json::from_value::<LeaseRequest>(serde_json::Value::Object(missing)).is_err());
}
