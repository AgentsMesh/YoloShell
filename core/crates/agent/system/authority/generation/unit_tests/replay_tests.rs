use super::fixture::*;
use crate::{EventKind, SessionStartSource};
use yolo_agent_contract::AgentState;
use yolo_hooks_runtime::ProcessIdentity;

#[test]
fn provisional_startup_becomes_recoverable_after_transcript_exists() {
    let fixture = Fixture::new();
    std::fs::remove_file(&fixture.transcript).unwrap();
    let started = fixture.apply(&fixture.start()).unwrap();
    assert_eq!(
        started.binding.recovery_availability,
        yolo_agent_contract::AgentRecoveryAvailability::Unavailable
    );
    std::fs::write(&fixture.transcript, b"{\"sessionId\":\"evidence\"}\n").unwrap();
    let running = fixture
        .apply(&fixture.request(2, EventKind::UserPromptSubmit, SESSION, &fixture.transcript))
        .unwrap();
    assert_eq!(
        running.binding.recovery_availability,
        yolo_agent_contract::AgentRecoveryAvailability::Available
    );
    assert!(running.transcript_identity.is_some());
}

#[test]
fn gaps_and_foreign_evidence_are_rejected_without_overwriting_authority() {
    let fixture = Fixture::new();
    let started = fixture.apply(&fixture.start()).unwrap();
    assert!(
        fixture
            .apply(&fixture.request(3, EventKind::UserPromptSubmit, SESSION, &fixture.transcript))
            .is_err()
    );
    let next = fixture.cwd.join(NEXT_TRANSCRIPT);
    std::fs::write(&next, b"{\"sessionId\":\"next\"}\n").unwrap();
    assert!(
        fixture
            .apply(&fixture.request(2, EventKind::UserPromptSubmit, NEXT_SESSION, &next))
            .is_err()
    );
    assert_eq!(
        fixture.store.load_in_namespace(&fixture.namespace).unwrap(),
        Some(started)
    );
}

#[test]
fn revocation_is_durable_and_blocks_resume() {
    let fixture = Fixture::new();
    let started = fixture.apply(&fixture.start()).unwrap();
    fixture
        .store
        .revoke_in_namespace(&fixture.namespace, fixture.scope.terminal_id, || Ok(()))
        .unwrap();
    let revoked = fixture
        .store
        .load_in_namespace(&fixture.namespace)
        .unwrap()
        .unwrap();
    assert!(revoked.revoked);
    assert!(!revoked.authorizes_resume(&started.binding));
    assert_eq!(
        revoked.binding.recovery_availability,
        yolo_agent_contract::AgentRecoveryAvailability::Unavailable
    );
    assert!(revoked.binding.state == AgentState::Unknown);
    assert_eq!(fixture.runtime().worker, fixture.worker);
    assert!(fixture.root.path().exists());
    assert_eq!(ProcessIdentity::new(42, 7).pid, 42);
}
