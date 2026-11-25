use super::support::*;
use std::time::Instant;
use uuid::Uuid;
use yolo_hub_client_channels::ControlError;
use yolo_hub_terminal_protocol::CreationQueryOutcome;
use yolo_tui_creation_recovery::{BlockedReason, CreationRecovery, RETRY_DELAY};

#[test]
fn strong_results_survive_disconnect_and_repeated_reads_until_exact_settlement() {
    let original = intent();
    let creation = creation(&original);
    for outcome in [
        CreationQueryOutcome::Ready { creation },
        CreationQueryOutcome::Finalized { creation },
        CreationQueryOutcome::Survivor { creation },
        CreationQueryOutcome::Archived {
            address: creation.address,
        },
    ] {
        let now = Instant::now();
        let mut recovery = CreationRecovery::default();
        recovery.retain(original.clone(), now).unwrap();
        let request = recovery.prepare(Some(Uuid::new_v4()), now).remove(FIRST);
        assert_eq!(requested(&request), original);
        assert!(!request.command.mutates());
        assert!(recovery.finished(&request, Ok(reply(&original, outcome.clone())), now));
        assert!(!request.validity.as_ref().unwrap().is_current());
        let resolved = recovery.resolutions();
        assert_eq!(resolved[FIRST].0, original);
        assert_eq!(resolved[FIRST].1.outcome(), &outcome);
        assert!(!recovery.retain(original.clone(), now).unwrap());
        assert!(recovery.prepare(None, now).is_empty());
        assert!(
            recovery
                .prepare(Some(Uuid::new_v4()), now + RETRY_DELAY)
                .is_empty()
        );
        assert!(!recovery.finished(&request, Err(ControlError::NotConnected), now));
        assert_eq!(recovery.resolutions(), resolved);
        assert!(recovery.settled(&original).unwrap());
        assert!(recovery.is_empty());
    }
}

#[test]
fn missing_and_pending_delay_readonly_queries_and_unknown_requires_explicit_retry() {
    let now = Instant::now();
    let generation = Some(Uuid::new_v4());
    let original = intent();
    let mut recovery = CreationRecovery::default();
    recovery.retain(original.clone(), now).unwrap();
    let mut next = now;
    for outcome in [
        CreationQueryOutcome::Missing {},
        CreationQueryOutcome::Pending {},
    ] {
        let request = recovery.prepare(generation, next).remove(FIRST);
        assert!(recovery.finished(&request, Ok(reply(&original, outcome)), next));
        assert!(recovery.resolutions().is_empty());
        assert!(recovery.prepare(generation, next).is_empty());
        next += RETRY_DELAY;
    }
    let request = recovery.prepare(generation, next).remove(FIRST);
    recovery.finished(
        &request,
        Ok(reply(&original, CreationQueryOutcome::Unknown {})),
        next,
    );
    assert_eq!(
        recovery.status().first_blocked,
        Some(BlockedReason::Unknown)
    );
    assert!(recovery.prepare(generation, next + RETRY_DELAY).is_empty());
    recovery.retry_blocked(next);
    let retried = recovery.prepare(generation, next).remove(FIRST);
    assert_eq!(requested(&retried), original);
    assert!(!retried.command.mutates());
}

#[test]
fn channel_task_failure_and_not_sent_retire_only_the_current_read_scope() {
    let now = Instant::now();
    let generation = Some(Uuid::new_v4());
    let mut recovery = CreationRecovery::default();
    recovery.retain(intent(), now).unwrap();
    let first = recovery.prepare(generation, now).remove(FIRST);
    assert!(recovery.not_sent(&first, now));
    assert!(!first.validity.as_ref().unwrap().is_current());
    let second = recovery
        .prepare(generation, now + RETRY_DELAY)
        .remove(FIRST);
    assert!(!recovery.not_sent(&first, now + RETRY_DELAY));
    assert!(recovery.finished(
        &second,
        Err(ControlError::QueryTaskFailed),
        now + RETRY_DELAY
    ));
    assert_eq!(recovery.status().blocked, usize::default());
    assert!(recovery.resolutions().is_empty());
    assert!(!second.validity.as_ref().unwrap().is_current());
}
