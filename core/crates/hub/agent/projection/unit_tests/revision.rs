use super::{fixture::record, view::view};
use crate::observation::revise;
use uuid::Uuid;
use yolo_hub_agent_protocol::*;

#[test]
fn revisions_are_stable_for_duplicate_observations_and_fail_closed_at_exhaustion() {
    let record = record(&view());
    let before = &record.entries.values().next().unwrap().snapshot;
    assert_eq!(revise(before, before.fields().clone()).unwrap(), *before);
    let mut changed = before.fields().clone();
    changed.activity = Activity::WaitingForInput;
    let after = revise(before, changed).unwrap();
    assert_eq!(
        after.cursor().sequence(),
        before.cursor().sequence() + FIRST_OBSERVATION_SEQUENCE
    );
    let mut exhausted = before.fields().clone();
    exhausted.cursor =
        ObservationCursor::new(before.cursor().target(), before.cursor().stream(), u64::MAX)
            .unwrap();
    let exhausted = AgentSnapshot::new(exhausted).unwrap();
    let mut changed = exhausted.fields().clone();
    changed.freshness = Freshness::Stale;
    assert!(revise(&exhausted, changed).is_err());
    let mut replacement = before.fields().clone();
    let mut target = before.cursor().target();
    target.execution_id = yolo_hub_identity::ExecutionId::new();
    replacement.cursor =
        ObservationCursor::new(target, Uuid::new_v4(), FIRST_OBSERVATION_SEQUENCE).unwrap();
    assert_eq!(
        revise(before, replacement).unwrap().cursor().sequence(),
        FIRST_OBSERVATION_SEQUENCE
    );
}
