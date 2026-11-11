use super::{fixture::record, view::view};
use crate::record::Record;
use yolo_hub_agent_protocol::{AgentSnapshot, Freshness, Reachability};
use yolo_hub_identity::{AgentId, HubId, RuntimeEpoch, TerminalId};

const LATEST_FIELD: &str = "latest";
const RETIRED_CURRENT_FIELD: &str = "current";

#[test]
fn logical_association_uses_only_the_latest_field() {
    let record = record(&view());
    let mut wire = serde_json::to_value(&record).unwrap();
    let latest = wire.as_object_mut().unwrap().remove(LATEST_FIELD).unwrap();
    wire[RETIRED_CURRENT_FIELD] = latest;
    assert!(serde_json::from_value::<Record>(wire).is_err());
}

#[test]
fn persisted_record_rejects_each_identity_and_metadata_mismatch() {
    let record = record(&view());
    assert!(record.validate(record.hub, record.directory).is_ok());
    let mutations: &[fn(&mut Record)] = &[
        |r| r.schema += 1,
        |r| r.hub = HubId::new(),
        |r| {
            let entry = r.entries.values_mut().next().unwrap();
            entry.runtime = RuntimeEpoch::new();
        },
        |r| {
            let entry = r.entries.values_mut().next().unwrap();
            entry.subject.terminal_id = TerminalId::new();
        },
        |r| {
            let entry = r.entries.values_mut().next().unwrap();
            entry.subject.kind = String::from("other");
        },
        |r| {
            let entry = r.entries.values_mut().next().unwrap();
            entry.subject.native_reference = String::from("other");
        },
        |r| {
            let entry = r.entries.values_mut().next().unwrap();
            entry.detected_at_ms = -1;
        },
        |r| {
            let entry = r.entries.values_mut().next().unwrap();
            entry.updated_at_ms = entry.detected_at_ms - 1;
        },
        |r| {
            let entry = r.entries.values_mut().next().unwrap();
            entry.updated_at_ms += 1;
        },
        |r| {
            let entry = r.entries.values_mut().next().unwrap();
            entry.process.as_mut().unwrap().pid = 0;
        },
        |r| {
            let entry = r.entries.values_mut().next().unwrap();
            entry.process.as_mut().unwrap().start_time = 0;
        },
        |r| {
            let entry = r.entries.values_mut().next().unwrap();
            entry.subject.data_home = String::from("relative");
        },
        |r| {
            *r.latest.values_mut().next().unwrap() = AgentId::new();
        },
    ];
    for mutate in mutations {
        let mut changed = record.clone();
        mutate(&mut changed);
        assert!(changed.validate(record.hub, record.directory).is_err());
    }
    assert!(record.validate(HubId::new(), record.directory).is_err());
    let mut wire = serde_json::to_value(&record).unwrap();
    wire["unchecked"] = true.into();
    assert!(serde_json::from_value::<Record>(wire).is_err());
}

#[test]
fn invalidation_keeps_execution_and_last_observed_time_without_claiming_current_liveness() {
    let mut record = record(&view());
    let id = *record.entries.keys().next().unwrap();
    let previous = record.entries[&id].snapshot.clone();
    record.invalidate(Some(TerminalId::new())).unwrap();
    assert_eq!(record.entries[&id].snapshot, previous);
    record.invalidate(None).unwrap();
    let after = &record.entries[&id].snapshot;
    assert_eq!(after.cursor().target(), previous.cursor().target());
    assert_eq!(
        after.fields().observed_unix_ms,
        previous.fields().observed_unix_ms
    );
    assert_eq!(after.fields().freshness, Freshness::Stale);
    assert_eq!(after.fields().reachability, Reachability::Unreachable);
    let after = after.clone();
    record.invalidate(None).unwrap();
    assert_eq!(record.entries[&id].snapshot, after);
}

#[test]
fn aliasing_two_agents_to_one_execution_is_rejected() {
    let mut record = record(&view());
    let mut duplicate = record.entries.values().next().unwrap().clone();
    let id = AgentId::new();
    let mut fields = duplicate.snapshot.fields().clone();
    let target = fields.cursor.target();
    fields.cursor = yolo_hub_agent_protocol::ObservationCursor::new(
        yolo_hub_identity::AgentTarget {
            address: yolo_hub_identity::AgentAddress {
                agent_id: id,
                ..target.address
            },
            ..target
        },
        fields.cursor.stream(),
        fields.cursor.sequence(),
    )
    .unwrap();
    duplicate.snapshot = AgentSnapshot::new(fields).unwrap();
    record.entries.insert(id, duplicate);
    assert!(record.validate(record.hub, record.directory).is_err());
}

#[test]
fn origin_directory_and_retention_bounds_are_checked_before_publication() {
    let original = record(&view());
    let mut relocated = original.clone();
    relocated.directory = record(&view()).directory;
    let record = original;
    assert!(relocated.validate(record.hub, record.directory).is_err());
    let mut changed = record.clone();
    let entry = changed.entries.values().next().unwrap().clone();
    for _ in 0..=crate::constants::MAX_AGENT_RECORDS {
        changed.entries.insert(AgentId::new(), entry.clone());
    }
    assert!(changed.validate(record.hub, record.directory).is_err());
    let mut changed = record.clone();
    let entry = changed.entries.values_mut().next().unwrap();
    let mut fields = entry.snapshot.fields().clone();
    let old = fields.cursor;
    let mut target = old.target();
    target.address.hub_id = HubId::new();
    fields.terminal.address.hub_id = target.address.hub_id;
    fields.cursor =
        yolo_hub_agent_protocol::ObservationCursor::new(target, old.stream(), old.sequence())
            .unwrap();
    entry.snapshot = AgentSnapshot::new(fields).unwrap();
    assert!(changed.validate(record.hub, record.directory).is_err());
}
