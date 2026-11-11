use crate::{constants::*, fixture::*};
use yolo_hub_client_access_protocol::*;
use yolo_hub_routing::{Right, Rights};

#[test]
fn every_right_is_independent_and_empty_requests_never_authorize() {
    let all = [
        Right::Observe,
        Right::TerminalRead,
        Right::TerminalWrite,
        Right::Lifecycle,
        Right::FileRead,
        Right::AgentControl,
    ];
    for granted in all {
        let mut current = grant(FIRST_INDEX);
        current.origins = vec![origin(FIRST_INDEX, granted)];
        for requested in all {
            assert_eq!(
                current.allows(hub(FIRST_INDEX), &rights(requested)),
                requested == granted
            );
        }
        assert!(!current.allows(hub(FIRST_INDEX), &Rights::default()));
        assert!(!current.allows(hub(SECOND_INDEX), &rights(granted)));
    }
}

#[test]
fn rights_cannot_be_unioned_across_origins() {
    let mut current = grant(FIRST_INDEX);
    current
        .origins
        .push(origin(SECOND_INDEX, Right::TerminalWrite));
    let combined = rights(Right::Observe).union(&rights(Right::TerminalWrite));
    assert!(!current.allows(hub(FIRST_INDEX), &combined));
    assert!(!current.allows(hub(SECOND_INDEX), &combined));
    current.origins[FIRST_INDEX].rights = combined.clone();
    assert!(current.allows(hub(FIRST_INDEX), &combined));
}

#[test]
fn expiry_is_strict_and_does_not_overflow_at_the_clock_boundary() {
    let mut current = grant(FIRST_INDEX);
    for now in [NOW_MS, EXPIRES_AT_MS - TIME_STEP_MS] {
        assert_eq!(current.validate_at(now), Ok(()));
    }
    for now in [EXPIRES_AT_MS, EXPIRES_AT_MS + TIME_STEP_MS, u64::MAX] {
        assert_eq!(current.validate_at(now), Err(AccessError::Expired));
    }
    current.expires_at_ms = u64::MAX;
    assert_eq!(current.validate_at(u64::MAX - TIME_STEP_MS), Ok(()));
    assert_eq!(current.validate_at(u64::MAX), Err(AccessError::Expired));
    current.expires_at_ms = INVALID_EXPIRY_MS;
    assert_eq!(current.validate_at(NOW_MS), Err(AccessError::Grant));
}

#[test]
fn malformed_public_fields_never_grant_rights() {
    let mut current = grant(FIRST_INDEX);
    current.identity.client_id = uuid::Uuid::nil();
    assert_eq!(current.validate(), Err(AccessError::Identity));
    assert!(!current.allows(hub(FIRST_INDEX), &rights(Right::Observe)));
    current = grant(FIRST_INDEX);
    current.origins.clear();
    assert_eq!(current.validate(), Err(AccessError::Grant));
    current = grant(FIRST_INDEX);
    current.origins[FIRST_INDEX].rights = Rights::default();
    assert_eq!(current.validate(), Err(AccessError::Grant));
    assert_eq!(
        current.origins[FIRST_INDEX].validate(),
        Err(AccessError::Grant)
    );
}

#[test]
fn duplicate_origins_are_rejected_even_with_disjoint_rights() {
    let mut current = grant(FIRST_INDEX);
    current
        .origins
        .push(origin(FIRST_INDEX, Right::TerminalWrite));
    assert_eq!(current.validate(), Err(AccessError::DuplicateOrigin));
    assert!(!current.allows(hub(FIRST_INDEX), &rights(Right::Observe)));
}

#[test]
fn origin_capacity_accepts_the_boundary_and_rejects_one_more() {
    let mut current = grant(FIRST_INDEX);
    current.origins = (FIRST_INDEX..MAX_CLIENT_ORIGINS)
        .map(|index| origin(index, Right::Observe))
        .collect();
    assert_eq!(current.validate(), Ok(()));
    current
        .origins
        .push(origin(MAX_CLIENT_ORIGINS, Right::Observe));
    assert_eq!(current.validate(), Err(AccessError::Capacity));
}
