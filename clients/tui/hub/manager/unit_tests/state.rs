use super::fixture::*;
use yolo_hub_management_client::ManagementError;
use yolo_hub_management_protocol::CONFIGURATION_REVISION_STEP;

#[test]
fn pending_mutation_and_conflict_cannot_be_automatically_resubmitted() {
    let mut manager = manager();
    edit(&mut manager);
    assert!(manager.dirty());
    let request = manager.proposal().unwrap();
    assert!(manager.proposal().is_none());
    manager.complete(request.token, Err(ManagementError::Conflict));
    assert!(manager.dirty() && manager.blocked);
    assert!(manager.proposal().is_none());
    assert!(manager.unresolved.is_none());
}

#[test]
fn unknown_delivery_only_resolves_against_the_exact_applied_receipt() {
    let mut manager = manager();
    edit(&mut manager);
    let original = manager.base.clone().unwrap();
    let request = manager.proposal().unwrap();
    let yolo_hub_rpc_protocol::Command::Configure { change } = request.command else {
        panic!()
    };
    manager.complete(request.token, Err(ManagementError::OutcomeUnknown));
    let check = manager.query(true);
    manager.complete(check.token, Ok(configuration(original.clone())));
    assert!(manager.dirty() && manager.blocked && manager.unresolved.is_some());
    let mut applied = original;
    let retained_socket = applied.peer_socket.clone();
    applied.state.revision += CONFIGURATION_REVISION_STEP;
    applied.state.last_change = change.request_id;
    applied.state.configuration = change.configuration;
    applied.peer_socket = retained_socket;
    let check = manager.query(true);
    manager.complete(check.token, Ok(configuration(applied)));
    assert!(!manager.dirty() && !manager.blocked && manager.unresolved.is_none());
}

#[test]
fn stale_callbacks_and_changed_local_key_do_not_replace_current_configuration() {
    let mut manager = manager();
    let before = manager.base.clone().unwrap();
    let stale = manager.query(false);
    let current = manager.query(false);
    manager.complete(stale.token, Ok(configuration(snapshot())));
    assert_eq!(manager.base, Some(before.clone()));
    assert!(manager.pending());
    let mut malicious = before.clone();
    let key = [2; yolo_hub_delegation::ED25519_PUBLIC_KEY_BYTES];
    malicious.state.identity = yolo_hub_delegation::PeerIdentity::new(manager.entry, key).unwrap();
    manager.complete(current.token, Ok(configuration(malicious)));
    assert_eq!(manager.base, Some(before));
}

#[test]
fn local_queue_rejection_preserves_draft_and_does_not_create_uncertain_delivery() {
    let mut manager = manager();
    edit(&mut manager);
    let request = manager.proposal().unwrap();
    manager.not_sent(uuid::Uuid::new_v4());
    assert!(manager.pending());
    manager.not_sent(request.token);
    assert!(!manager.pending() && manager.unresolved.is_none());
    assert!(manager.proposal().is_some());
}

#[test]
fn inactive_persisted_configuration_can_be_explicitly_activated_without_inventing_edits() {
    let snapshot = snapshot();
    let (mut manager, request) = crate::Manager::open(snapshot.state.identity.hub());
    let mut inactive = snapshot;
    inactive.applied = false;
    inactive.peer_socket = None;
    manager.complete(request.token, Ok(configuration(inactive)));
    assert!(!manager.dirty());
    assert!(manager.can_apply());
    assert!(manager.proposal().is_some());
}

#[test]
fn reload_rejects_revision_rollback_and_same_revision_equivocation() {
    let mut manager = manager();
    manager.base.as_mut().unwrap().state.revision += CONFIGURATION_REVISION_STEP;
    let before = manager.base.clone();
    for equivocate in [false, true] {
        let mut reply = before.clone().unwrap();
        if equivocate {
            reply.state.last_change = uuid::Uuid::new_v4();
        } else {
            reply.state.revision -= CONFIGURATION_REVISION_STEP;
        }
        let request = manager.query(false);
        manager.complete(request.token, Ok(configuration(reply)));
        assert_eq!(manager.base, before);
        assert!(manager.blocked);
    }
}
