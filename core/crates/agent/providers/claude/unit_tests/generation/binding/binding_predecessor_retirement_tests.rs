use super::super::{
    ClaudeGenerationAuthority, GenerationBegin, GenerationEvent, GenerationRecoveryEvent,
    SessionStartSource,
};
use super::support::{Fixture, binding};
use crate::agent::{AgentBindingStore, AgentState};
use uuid::Uuid;

const SUCCESSOR_IDENTITY_OFFSET: u64 = 100;
const EVENT_INCREMENT: u64 = 1;

#[test]
fn an_active_only_store_does_not_inspect_unrelated_binding_state() {
    let fixture = Fixture::new();
    fixture
        .store
        .begin_in_namespace(&fixture.lease, &fixture.begin)
        .unwrap();
    std::fs::create_dir(fixture.binding_path()).unwrap();
    assert!(
        fixture
            .store
            .dead_parked_in_namespace(&fixture.lease)
            .unwrap()
            .is_empty()
    );
}

struct Handoff {
    fixture: Fixture,
    previous: ClaudeGenerationAuthority,
    successor: GenerationBegin,
    bindings: AgentBindingStore,
}

impl Handoff {
    fn new() -> Self {
        let fixture = Fixture::new();
        fixture
            .store
            .begin_in_namespace(&fixture.lease, &fixture.begin)
            .unwrap();
        let event = fixture.start(&fixture.begin.provider_session_id, SessionStartSource::Fresh);
        let previous = fixture
            .store
            .apply_in_namespace(&fixture.lease, fixture.authentication(), &event)
            .unwrap()
            .0;
        let bindings = AgentBindingStore::new(fixture.binding_path());
        bindings
            .save_authoritative_in_namespace(
                &fixture.lease,
                &binding(&previous),
                &previous,
                fixture.runtime(),
            )
            .unwrap();
        let mut successor = fixture.begin.clone();
        successor.invocation_id = Uuid::new_v4();
        successor.hook_token = Uuid::new_v4();
        successor.helper_capability = Uuid::new_v4();
        successor.runtime_epoch = Uuid::new_v4();
        successor.process.pid += u32::try_from(SUCCESSOR_IDENTITY_OFFSET).unwrap();
        successor.process.start_time += SUCCESSOR_IDENTITY_OFFSET;
        successor.worker.pid += u32::try_from(SUCCESSOR_IDENTITY_OFFSET).unwrap();
        successor.worker.start_time += SUCCESSOR_IDENTITY_OFFSET;
        fixture
            .store
            .begin_in_namespace(&fixture.lease, &successor)
            .unwrap();
        Self {
            fixture,
            previous,
            successor,
            bindings,
        }
    }

    fn mark_previous_gone(&self) -> crate::process::ProcessIdentityRelationGuard {
        crate::process::install_test_process_identity_relation(
            self.previous.process.pid,
            self.previous.process.start_time,
            crate::process::ProcessIdentityRelation::Gone,
        )
    }

    fn assert_previous_retained(&self) {
        assert_eq!(
            self.fixture
                .store
                .load_invocation_in_namespace(&self.fixture.lease, self.previous.invocation_id)
                .unwrap(),
            Some(self.previous.clone())
        );
        assert_eq!(
            self.bindings
                .read_raw_in_namespace(&self.fixture.lease)
                .unwrap(),
            Some(binding(&self.previous))
        );
    }
}

#[test]
fn dead_predecessor_stays_pinned_until_the_successor_binding_commits() {
    let handoff = Handoff::new();
    let _gone = handoff.mark_previous_gone();
    let store = &handoff.fixture.store;
    let namespace = &handoff.fixture.lease;
    assert!(
        store
            .dead_parked_in_namespace(namespace)
            .unwrap()
            .is_empty()
    );
    assert!(
        store
            .retire_parked_in_namespace(
                namespace,
                handoff.previous.authentication(),
                &handoff.previous
            )
            .is_err()
    );
    handoff.assert_previous_retained();

    let event = handoff.fixture.start(
        &handoff.successor.provider_session_id,
        SessionStartSource::Fresh,
    );
    let successor = store
        .apply_in_namespace(
            namespace,
            handoff.fixture.authentication_for(&handoff.successor),
            &event,
        )
        .unwrap()
        .0;
    handoff
        .bindings
        .save_authoritative_in_namespace(
            namespace,
            &binding(&successor),
            &successor,
            handoff.fixture.runtime_for(&handoff.successor),
        )
        .unwrap();
    assert_eq!(
        store.dead_parked_in_namespace(namespace).unwrap(),
        vec![handoff.previous.clone()]
    );
    store
        .retire_parked_in_namespace(
            namespace,
            handoff.previous.authentication(),
            &handoff.previous,
        )
        .unwrap();
    assert!(
        store
            .load_invocation_in_namespace(namespace, handoff.previous.invocation_id)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        handoff.bindings.read_raw_in_namespace(namespace).unwrap(),
        Some(binding(&successor))
    );
}

#[test]
fn parked_recovery_cannot_mutate_the_last_published_binding_predecessor() {
    let handoff = Handoff::new();
    let _gone = handoff.mark_previous_gone();
    let event = GenerationRecoveryEvent {
        sequence: handoff.previous.last_event_sequence + EVENT_INCREMENT,
        event: GenerationEvent::StateChanged {
            event_id: Uuid::new_v4(),
            provider_session_id: handoff.previous.provider_session_id.clone(),
            state: AgentState::WaitingForInput,
        },
        transcript_identity: None,
    };
    assert!(
        handoff
            .fixture
            .store
            .apply_parked_recovery_in_namespace(
                &handoff.fixture.lease,
                handoff.previous.authentication(),
                &handoff.previous,
                &[event]
            )
            .is_err()
    );
    handoff.assert_previous_retained();
}
