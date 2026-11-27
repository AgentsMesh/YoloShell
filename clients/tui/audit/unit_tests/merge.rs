use crate::merge::merge;
use std::path::PathBuf;
use tempfile::TempDir;
use yolo_hub_audit::{AuditOutcome, AuditRecord};
use yolo_hub_identity::HubId;

fn record(hub: HubId, sequence: u64, at_ms: i64, method: &str) -> AuditRecord {
    AuditRecord {
        sequence,
        at_ms,
        principal: "local".into(),
        method: method.into(),
        destination: hub,
        target: None,
        request_id: None,
        outcome: AuditOutcome::Accepted,
    }
}

fn source(directory: &TempDir, name: &str, records: &[AuditRecord]) -> PathBuf {
    let path = directory.path().join(name);
    let mut body = String::new();
    for record in records {
        body.push_str(&serde_json::to_string(record).unwrap());
        body.push('\n');
    }
    std::fs::write(&path, body).unwrap();
    path
}

#[test]
fn merge_dedupes_identical_records_and_orders_by_time() {
    let directory = TempDir::new().unwrap();
    let hub = HubId::new();
    let first = record(hub, 1, 10, "alpha");
    let second = record(hub, 2, 20, "beta");
    let third = record(hub, 3, 30, "gamma");
    let left = source(&directory, "left.jsonl", &[first.clone(), second.clone()]);
    let right = source(&directory, "right.jsonl", &[first, third]);
    let merged = merge(&[left, right], None).unwrap();
    assert_eq!(merged.duplicates, 1);
    assert_eq!(merged.conflicts, 0);
    assert_eq!(merged.unsigned, 4);
    assert_eq!(
        merged
            .records
            .iter()
            .map(|(entry, _)| entry.record.sequence)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
}

#[test]
fn merge_reports_conflicting_sequence_and_rejects_missing_sources() {
    let directory = TempDir::new().unwrap();
    let hub = HubId::new();
    let original = record(hub, 1, 10, "alpha");
    let conflict = record(hub, 1, 10, "beta");
    let left = source(&directory, "left.jsonl", &[original]);
    let right = source(&directory, "right.jsonl", &[conflict]);
    let merged = merge(&[left, right], None).unwrap();
    assert_eq!(merged.conflicts, 1);
    assert_eq!(merged.records.len(), 1);
    assert_eq!(merged.records[0].0.record.method, "alpha");
    assert!(merge(&[directory.path().join("absent.jsonl")], None).is_err());
}
