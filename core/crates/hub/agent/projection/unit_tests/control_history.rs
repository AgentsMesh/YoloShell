use super::{constants::*, control_support::observation, fixture::record, view::view};
use crate::{constants::MAX_AGENT_RECORDS, control::ControlWitness};
use yolo_hub_agent_protocol::{AgentSnapshot, Freshness, ObservationCursor, Reachability};
use yolo_hub_identity::{AgentId, ExecutionId};

#[test]
fn retained_control_executions_share_global_projection_capacity_without_eviction() {
    let source = view();
    let mut record = record(&source);
    let current = record.entries.values().next().unwrap().clone();
    // Build well-formed stale history without thousands of redundant disk writes.
    while record.entries.len() < MAX_AGENT_RECORDS {
        let mut entry = current.clone();
        let mut fields = entry.snapshot.fields().clone();
        let mut target = fields.cursor.target();
        target.address.agent_id = AgentId::new();
        target.execution_id = ExecutionId::new();
        fields.cursor =
            ObservationCursor::new(target, uuid::Uuid::new_v4(), IDENTITY_STEP).unwrap();
        fields.freshness = Freshness::Stale;
        fields.reachability = Reachability::Unreachable;
        entry.snapshot = AgentSnapshot::new(fields).unwrap();
        record.entries.insert(target.address.agent_id, entry);
    }
    record.validate(record.hub, record.directory).unwrap();
    let mut previous = current.snapshot.cursor().target();
    previous.execution_id = ExecutionId::new();
    let witness = ControlWitness {
        cursor: ObservationCursor::new(previous, uuid::Uuid::new_v4(), IDENTITY_STEP).unwrap(),
        witness: observation(&source, IDENTITY_STEP).witness().clone(),
    };
    record
        .retired_controls
        .insert(previous.execution_id, witness);
    assert!(record.validate(record.hub, record.directory).is_err());
    let removable = *record
        .entries
        .keys()
        .find(|id| **id != previous.address.agent_id)
        .unwrap();
    record.entries.remove(&removable);
    record.validate(record.hub, record.directory).unwrap();
    assert!(record.retired_controls.contains_key(&previous.execution_id));
}
