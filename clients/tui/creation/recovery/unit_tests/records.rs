use super::support::*;
use crate::*;
use std::time::Instant;
use uuid::Uuid;
use yolo_hub_identity::HubId;
use yolo_tui_creation_journal::MAX_PENDING_CREATIONS;

#[test]
fn retain_and_settle_require_exact_intent_across_origins_and_request_aliases() {
    let now = Instant::now();
    let original = intent();
    let mut recovery = CreationRecovery::default();
    assert!(recovery.retain(original.clone(), now).unwrap());
    assert!(!recovery.retain(original.clone(), now).unwrap());
    for wrong in [
        changed(&original, |fields| fields.request_id = Uuid::new_v4()),
        changed(&original, |fields| fields.creation_token = Uuid::new_v4()),
        changed(&original, |fields| fields.cwd = OTHER_DIRECTORY.into()),
        {
            let mut value = original.clone();
            value.origin = HubId::new();
            value
        },
    ] {
        assert!(recovery.contains_request(&wrong.request));
        assert!(recovery.retain(wrong.clone(), now).is_err());
        assert!(recovery.settled(&wrong).is_err());
        assert_eq!(recovery.len(), ONE);
    }
    assert!(!recovery.contains_request(&intent().request));
    assert!(!recovery.settled(&intent()).unwrap());
    assert!(recovery.settled(&original).unwrap());
    assert!(recovery.is_empty());
}

#[test]
fn capacity_is_shared_by_waiting_and_resolved_records_until_durable_settlement() {
    let now = Instant::now();
    let mut recovery = CreationRecovery::default();
    let records: Vec<_> = (0..MAX_PENDING_CREATIONS).map(|_| intent()).collect();
    for record in &records {
        recovery.retain(record.clone(), now).unwrap();
    }
    assert!(!recovery.retain(records[FIRST].clone(), now).unwrap());
    assert!(recovery.retain(intent(), now).is_err());
    let request = recovery.prepare(Some(Uuid::new_v4()), now).remove(FIRST);
    assert!(recovery.finished(&request, Ok(ready(&requested(&request))), now));
    assert!(recovery.retain(intent(), now).is_err());
    assert!(recovery.settled(&requested(&request)).unwrap());
    assert!(recovery.retain(intent(), now).unwrap());
    assert_eq!(recovery.len(), MAX_PENDING_CREATIONS);
}

#[test]
fn forged_deadline_inside_the_right_scope_cannot_complete_an_attempt() {
    let now = Instant::now();
    let original = intent();
    let token = original.request.fields().creation_token;
    let mut recovery = CreationRecovery::default();
    recovery.retain(original.clone(), now).unwrap();
    let mut request = recovery.prepare(Some(Uuid::new_v4()), now).remove(FIRST);
    let attempt = recovery.entries[&token].attempt.as_ref().unwrap();
    request.validity = Some(
        attempt
            .scope
            .validity_until(attempt.validity.deadline() + RETRY_DELAY),
    );
    assert!(!recovery.finished(&request, Ok(ready(&original)), now));
    assert_eq!(recovery.status().in_flight, ONE);
    assert!(recovery.resolutions().is_empty());
}
