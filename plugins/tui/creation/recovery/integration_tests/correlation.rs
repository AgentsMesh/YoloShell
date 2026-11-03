use super::support::*;
use std::time::Instant;
use uuid::Uuid;
use yolo_hub_client_channels::{RPC_TIMEOUT, RequestScope};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{CreationQueryOutcome, CreationQueryRequest};
use yolo_tui_creation_recovery::{BlockedReason, CreationRecovery, RETRY_DELAY};

#[test]
fn foreign_generation_scope_origin_and_changed_request_cannot_resolve_an_attempt() {
    let now = Instant::now();
    let original = intent();
    let mut recovery = CreationRecovery::default();
    recovery.retain(original.clone(), now).unwrap();
    let request = recovery.prepare(Some(Uuid::new_v4()), now).remove(FIRST);
    let foreign_scope = RequestScope::default();
    for wrong in [
        {
            let mut value = request.clone();
            value.generation = Some(Uuid::new_v4());
            value
        },
        {
            let mut value = request.clone();
            value.destination = HubId::new();
            value
        },
        {
            let mut value = request.clone();
            value.validity = None;
            value
        },
        {
            let mut value = request.clone();
            value.validity = Some(foreign_scope.validity_until(now + RPC_TIMEOUT));
            value
        },
        {
            let mut value = request.clone();
            value.command = Command::QueryCreation {
                request: CreationQueryRequest::new(
                    changed(&original, |fields| fields.cwd = OTHER_DIRECTORY.into()).request,
                ),
            };
            value
        },
        {
            let mut value = request.clone();
            value.command = Command::Create {
                request: original.request.clone(),
            };
            value
        },
    ] {
        assert!(!recovery.finished(&wrong, Ok(ready(&original)), now));
        assert!(!recovery.not_sent(&wrong, now));
        assert!(recovery.resolutions().is_empty());
    }
    assert!(recovery.finished(&request, Ok(ready(&original)), now));
}

#[test]
fn expired_and_disconnected_attempts_cannot_override_a_new_generation() {
    let now = Instant::now();
    let original = intent();
    let mut recovery = CreationRecovery::default();
    recovery.retain(original.clone(), now).unwrap();
    let first = recovery.prepare(Some(Uuid::new_v4()), now).remove(FIRST);
    let expires = first.validity.as_ref().unwrap().deadline();
    assert_eq!(expires, now + RPC_TIMEOUT);
    assert!(!recovery.finished(&first, Ok(ready(&original)), expires));
    assert!(recovery.prepare(first.generation, expires).is_empty());
    let second = recovery
        .prepare(first.generation, expires + RETRY_DELAY)
        .remove(FIRST);
    assert!(!first.validity.as_ref().unwrap().is_current());
    assert!(recovery.prepare(None, expires + RETRY_DELAY).is_empty());
    assert!(!second.validity.as_ref().unwrap().is_current());
    let current = recovery
        .prepare(Some(Uuid::new_v4()), expires + RETRY_DELAY)
        .remove(FIRST);
    for old in [&first, &second] {
        assert!(!recovery.finished(old, Ok(ready(&original)), expires + RETRY_DELAY));
    }
    assert!(recovery.finished(&current, Ok(ready(&original)), expires + RETRY_DELAY));
}

#[test]
fn wrong_reply_token_origin_and_reply_kind_preserve_the_intent_and_block() {
    let original = intent();
    let foreign = intent();
    for result in [
        ready(&foreign),
        reply(
            &original,
            CreationQueryOutcome::Ready {
                creation: creation(&foreign),
            },
        ),
        Reply::Completed {},
    ] {
        let now = Instant::now();
        let mut recovery = CreationRecovery::default();
        recovery.retain(original.clone(), now).unwrap();
        let request = recovery.prepare(Some(Uuid::new_v4()), now).remove(FIRST);
        assert!(recovery.finished(&request, Ok(result), now));
        assert_eq!(
            recovery.status().first_blocked,
            Some(BlockedReason::InvalidResponse)
        );
        assert!(recovery.contains_request(&original.request));
        assert!(recovery.resolutions().is_empty());
    }
}
