use super::support::*;
use std::time::Instant;
use uuid::Uuid;
use yolo_hub_client_channels::MAX_QUERY_CHANNELS;
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::{CreationQueryOutcome, PublicError};
use yolo_tui_creation_recovery::{BlockedReason, CreationRecovery, RETRY_DELAY};

#[test]
fn query_slots_are_bounded_and_untouched_records_precede_retries() {
    let now = Instant::now();
    let generation = Some(Uuid::new_v4());
    let mut recovery = CreationRecovery::default();
    for _ in 0..MAX_QUERY_CHANNELS + ONE {
        recovery.retain(intent(), now).unwrap();
    }
    assert!(recovery.prepare(None, now).is_empty());
    assert!(recovery.prepare(Some(Uuid::nil()), now).is_empty());
    let batch = recovery.prepare(generation, now);
    assert_eq!(batch.len(), MAX_QUERY_CHANNELS);
    assert!(recovery.prepare(generation, now).is_empty());
    for request in &batch {
        recovery.not_sent(request, now);
    }
    let next = recovery.prepare(generation, now + RETRY_DELAY);
    assert_eq!(next.len(), MAX_QUERY_CHANNELS);
    assert!(
        batch
            .iter()
            .all(|old| requested(old) != requested(&next[FIRST]))
    );
    assert_eq!(recovery.status().in_flight, MAX_QUERY_CHANNELS);
    assert_eq!(recovery.status().pending, ONE);
}

#[test]
fn permanent_rejection_requires_explicit_query_retry_and_exact_settlement_retires_scope() {
    let now = Instant::now();
    let generation = Some(Uuid::new_v4());
    let original = intent();
    let mut recovery = CreationRecovery::default();
    recovery.retain(original.clone(), now).unwrap();
    let request = recovery.prepare(generation, now).remove(FIRST);
    recovery.finished(
        &request,
        Ok(Reply::Error {
            code: PublicError::Unauthorized,
        }),
        now,
    );
    assert_eq!(
        recovery.status().first_blocked,
        Some(BlockedReason::Rejected(PublicError::Unauthorized))
    );
    assert!(recovery.prepare(generation, now + RETRY_DELAY).is_empty());
    recovery.retry_blocked(now);
    let retry = recovery.prepare(generation, now).remove(FIRST);
    assert!(recovery.settled(&original).unwrap());
    assert!(!retry.validity.as_ref().unwrap().is_current());
    assert!(!recovery.finished(&retry, Ok(ready(&original)), now));
}

#[test]
fn status_buckets_are_disjoint_and_read_only_results_are_not_consumed() {
    let now = Instant::now();
    let mut recovery = CreationRecovery::default();
    for _ in 0..MAX_QUERY_CHANNELS {
        recovery.retain(intent(), now).unwrap();
    }
    let mut batch = recovery.prepare(Some(Uuid::new_v4()), now);
    let resolved = batch.pop().unwrap();
    let blocked = batch.pop().unwrap();
    let waiting = batch.pop().unwrap();
    recovery.finished(&resolved, Ok(ready(&requested(&resolved))), now);
    recovery.finished(
        &blocked,
        Ok(reply(
            &requested(&blocked),
            CreationQueryOutcome::Unknown {},
        )),
        now,
    );
    recovery.not_sent(&waiting, now);
    let status = recovery.status();
    assert_eq!(
        (
            status.pending,
            status.in_flight,
            status.resolved,
            status.blocked
        ),
        (ONE, ONE, ONE, ONE)
    );
    assert_eq!(
        status.pending + status.in_flight + status.resolved + status.blocked,
        recovery.len()
    );
    assert_eq!(recovery.resolutions(), recovery.resolutions());
    let debug = format!("{resolved:?}");
    assert!(!debug.contains(DIRECTORY));
    assert!(
        !debug.contains(
            &requested(&resolved)
                .request
                .fields()
                .creation_token
                .to_string()
        )
    );
    drop(recovery);
    assert!(
        batch
            .iter()
            .all(|request| !request.validity.as_ref().unwrap().is_current())
    );
}
