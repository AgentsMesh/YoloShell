use super::constants::{RENEW_AFTER_DIVISOR, code};
use super::{LeaseAuthority, LeaseReason, LeaseRequest, LeaseTransition, LeaseTransitionKind};
use uuid::Uuid;
use yolo_terminal_model::{WriterCapability, now_ms};

#[path = "capability.rs"]
mod capability;

const TEST_TTL_MS: u64 = 9_000;
const PAST_OFFSET_MS: i64 = 60_000;
const STALE_GENERATION_OFFSET: u64 = 1;

pub(super) fn request(holder_id: &str, instance_id: &str) -> LeaseRequest {
    LeaseRequest {
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        holder_id: holder_id.into(),
        ttl_ms: Some(TEST_TTL_MS),
        generation: None,
        controller_instance_id: Some(instance_id.into()),
        writer_capability: WriterCapability::generate(),
    }
}

#[test]
fn live_incarnations_conflict_and_explicit_takeover_is_fenced() {
    let mut authority = LeaseAuthority::default();
    let mut transitions = Vec::new();
    let mut emit = |transition| transitions.push(transition);
    let first_request = request("desktop", "main-1");
    let first = authority.acquire(&first_request, false, &mut emit).unwrap();
    assert_eq!(first.ttl_ms, TEST_TTL_MS);
    assert_eq!(first.renew_after_ms, TEST_TTL_MS / RENEW_AFTER_DIVISOR);
    assert_eq!(first.controller_instance_id.as_deref(), Some("main-1"));
    assert_eq!(
        authority
            .acquire(&first_request, false, &mut emit)
            .unwrap()
            .generation,
        first.generation
    );
    assert_eq!(
        authority
            .acquire(&first_request, true, &mut emit)
            .unwrap()
            .generation,
        first.generation
    );

    let mut replacement_request = request("desktop", "main-2");
    let conflict = authority
        .acquire(&replacement_request, false, &mut emit)
        .unwrap_err();
    assert_eq!(conflict.code, code::HELD_BY_OTHER);
    assert_eq!(
        authority.snapshot(&mut emit).unwrap().generation,
        first.generation
    );
    assert_eq!(
        authority
            .acquire(&replacement_request, true, &mut emit)
            .unwrap_err()
            .code,
        code::STALE_GENERATION
    );
    replacement_request.generation = Some(first.generation);
    let replacement = authority
        .acquire(&replacement_request, true, &mut emit)
        .unwrap();
    assert!(replacement.generation > first.generation);
    let mut stale = first_request;
    stale.generation = Some(first.generation);
    let error = authority.release(&stale, &mut emit).unwrap_err();
    assert_eq!(error.code, code::CONTROLLER_INSTANCE_MISMATCH);
    assert_eq!(
        error.details.as_ref().unwrap()[yolo_wire::field::HOLDER_ID],
        "desktop"
    );
    assert_eq!(
        error.details.as_ref().unwrap()[yolo_wire::field::GENERATION],
        replacement.generation
    );
    assert_eq!(
        authority.snapshot(&mut emit).unwrap().generation,
        replacement.generation
    );
}

#[test]
fn precise_errors_use_monotonic_expiration() {
    let mut authority = LeaseAuthority::default();
    let mut emit = |_| {};
    let owner = request("owner", "main");
    let lease = authority.acquire(&owner, false, &mut emit).unwrap();
    let held = authority
        .acquire(&request("other", "other-main"), false, &mut emit)
        .unwrap_err();
    assert_eq!(held.code, code::HELD_BY_OTHER);
    let mut stale_takeover = request("other", "other-main");
    stale_takeover.generation = Some(lease.generation + STALE_GENERATION_OFFSET);
    assert_eq!(
        authority
            .acquire(&stale_takeover, true, &mut emit)
            .unwrap_err()
            .code,
        code::STALE_GENERATION
    );

    let mut stale = owner.clone();
    stale.generation = Some(lease.generation + STALE_GENERATION_OFFSET);
    assert_eq!(
        authority.renew(&stale, &mut emit).unwrap_err().code,
        code::STALE_GENERATION
    );
    authority.set_wall_expiry_for_test(now_ms() - PAST_OFFSET_MS);
    authority
        .validate_authorization(lease.generation, &lease.writer_capability, &mut emit)
        .unwrap();
    authority.expire_for_test();
    let mut expired = owner;
    expired.generation = Some(lease.generation);
    assert_eq!(
        authority.renew(&expired, &mut emit).unwrap_err().code,
        code::LEASE_EXPIRED
    );
    assert_eq!(
        authority.renew(&expired, &mut emit).unwrap_err().code,
        code::NOT_HOLDER
    );
    let replacement = authority.acquire(&expired, false, &mut emit).unwrap();
    expired.generation = Some(replacement.generation);
    authority.expire_for_test();
    assert_eq!(
        authority.release(&expired, &mut emit).unwrap_err().code,
        code::LEASE_EXPIRED
    );
}

#[test]
fn authority_emits_acquire_takeover_release_and_expiry() {
    let mut authority = LeaseAuthority::default();
    let mut transitions: Vec<LeaseTransition> = Vec::new();
    let mut emit = |transition| transitions.push(transition);
    let first = authority
        .acquire(&request("first", "first-main"), false, &mut emit)
        .unwrap();
    let mut second_request = request("second", "second-main");
    second_request.generation = Some(first.generation);
    let second = authority.acquire(&second_request, true, &mut emit).unwrap();
    second_request.generation = Some(second.generation);
    authority.release(&second_request, &mut emit).unwrap();
    let expiring = authority
        .acquire(&second_request, false, &mut emit)
        .unwrap();
    authority.expire_for_test();
    assert!(authority.snapshot(&mut emit).is_none());

    assert_eq!(transitions.len(), 6);
    assert_eq!(transitions[0].kind, LeaseTransitionKind::Acquired);
    assert_eq!(transitions[1].kind, LeaseTransitionKind::Revoked);
    assert_eq!(transitions[1].reason, Some(LeaseReason::TakenOver));
    assert_eq!(transitions[2].lease.generation, second.generation);
    assert_eq!(transitions[3].reason, Some(LeaseReason::Released));
    assert_eq!(transitions[4].lease.generation, expiring.generation);
    assert_eq!(transitions[5].kind, LeaseTransitionKind::Expired);
}

#[test]
fn request_rejects_legacy_generation_alias() {
    let error = serde_json::from_value::<LeaseRequest>(serde_json::json!({
        (yolo_wire::field::TERMINAL_ID): Uuid::new_v4(),
        (yolo_wire::field::HOLDER_ID): "legacy",
        (yolo_wire::field::TTL_MS): 5_000,
        (yolo_wire::field::LEASE_GENERATION): 42,
        (yolo_wire::field::WRITER_CAPABILITY): WriterCapability::generate(),
    }))
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("unknown field `lease_generation`")
    );
}
