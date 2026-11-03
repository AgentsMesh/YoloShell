use super::*;
use std::collections::BTreeSet;

const VALID_EVENTS: &str = "populated 1\nfrozen 1\npressure 0\n";
const PID_ONE: u32 = 101;
const PID_TWO: u32 = 202;

#[test]
fn event_parser_accepts_required_fields_and_ignores_known_shape_extensions() {
    let events = parse_events(VALID_EVENTS).unwrap();
    assert!(events.populated);
    assert!(events.frozen);
}

#[test]
fn event_parser_rejects_missing_duplicate_and_malformed_fields() {
    for content in [
        "populated 1\n",
        "frozen 1\n",
        "populated 1\npopulated 1\nfrozen 1\n",
        "populated 1\nfrozen 1\nfrozen 1\n",
        "populated 2\nfrozen 1\n",
        "populated 1 extra\nfrozen 1\n",
        "populated\nfrozen 1\n",
    ] {
        assert!(matches!(
            parse_events(content),
            Err(LinuxContainmentError::Identity(EVENTS_MALFORMED_DETAIL))
        ));
    }
}

#[test]
fn frozen_observation_requires_stable_membership_and_events() {
    let one = BTreeSet::from([PID_ONE]);
    let two = BTreeSet::from([PID_TWO]);
    let frozen_populated = Events {
        populated: true,
        frozen: true,
    };
    assert!(validate_observation(frozen_populated, &one, &one, frozen_populated).is_ok());
    for (before, first, second, after) in [
        (frozen_populated, &one, &two, frozen_populated),
        (
            Events {
                populated: true,
                frozen: false,
            },
            &one,
            &one,
            frozen_populated,
        ),
        (
            frozen_populated,
            &one,
            &one,
            Events {
                populated: true,
                frozen: false,
            },
        ),
        (
            frozen_populated,
            &one,
            &one,
            Events {
                populated: false,
                frozen: true,
            },
        ),
    ] {
        assert!(matches!(
            validate_observation(before, first, second, after),
            Err(LinuxContainmentError::Identity(MEMBERSHIP_CHANGED_DETAIL))
        ));
    }
}

#[test]
fn empty_frozen_observation_requires_unpopulated_event() {
    let empty = BTreeSet::new();
    let frozen_empty = Events {
        populated: false,
        frozen: true,
    };
    assert!(validate_observation(frozen_empty, &empty, &empty, frozen_empty).is_ok());
    assert!(
        validate_observation(
            Events {
                populated: true,
                frozen: true,
            },
            &empty,
            &empty,
            frozen_empty,
        )
        .is_err()
    );
}

#[test]
fn unstable_freeze_observations_remain_pending_without_resetting_the_deadline() {
    let started = Instant::now();
    assert!(
        pending_before_deadline(Some(started), started)
            .unwrap()
            .is_none()
    );
    assert!(pending_before_deadline(None, started).unwrap().is_none());
    assert!(matches!(
        pending_before_deadline(Some(started), started + FREEZE_TIMEOUT),
        Err(LinuxContainmentError::Unavailable(FREEZE_TIMEOUT_DETAIL)),
    ));
}
