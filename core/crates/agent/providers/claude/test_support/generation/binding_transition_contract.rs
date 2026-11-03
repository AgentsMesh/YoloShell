use super::durable_state_publication_tests::published_fixture;
use super::support::Fixture;
use crate::agent::claude::generation::{
    ClaudeGenerationAuthority, GenerationEvent, GenerationRecoveryEvent, SessionStartSource,
    adversarial_test_support::prepare_durable_state_for_test,
    authority_set::ClaudeInvocationAuthorities,
    binding_transition::{BindingTransitionProof, PendingBindingTransition},
    constants::REVISION_INCREMENT,
    record,
};
use crate::agent::{AgentBinding, AgentState};
#[path = "binding_transition_contract_constants.rs"]
mod constants;
pub use constants::*;

pub struct PendingCase {
    pub fixture: Fixture,
    pub set: ClaudeInvocationAuthorities,
    pub authority: ClaudeGenerationAuthority,
}

impl PendingCase {
    pub fn state(state: AgentState) -> Self {
        let fixture = published_fixture();
        prepare_durable_state_for_test(&fixture.binding_path(), state).unwrap();
        Self::capture(fixture)
    }

    pub fn restarted() -> Self {
        let fixture = published_fixture();
        let current = fixture
            .store
            .load_in_namespace(&fixture.lease)
            .unwrap()
            .unwrap();
        let next_session = uuid::Uuid::new_v4().to_string();
        let event = fixture.start(&next_session, SessionStartSource::Clear);
        let GenerationEvent::SessionStart {
            transcript_path, ..
        } = &event
        else {
            unreachable!()
        };
        let identity = crate::paths::identity::ObjectIdentity::from_file(
            &std::fs::File::open(transcript_path).unwrap(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(current.binding_revision, Some(current.revision));
        assert!(fixture.binding_path().is_file());
        let (_, applied) = fixture
            .store
            .apply_recovery_in_namespace(
                &fixture.lease,
                fixture.authentication(),
                &current,
                &[GenerationRecoveryEvent {
                    sequence: current.last_event_sequence + REVISION_INCREMENT,
                    event,
                    transcript_identity: Some(identity),
                }],
            )
            .unwrap();
        assert!(applied);
        Self::capture(fixture)
    }

    fn capture(fixture: Fixture) -> Self {
        let mut set: ClaudeInvocationAuthorities =
            fixture.lease.read_json(fixture.store.path()).unwrap();
        let transition = set.pending_binding_transition.as_mut().unwrap();
        transition.predecessor.binding.detected_at_ms = DETECTED;
        transition.predecessor.binding.updated_at_ms = PREVIOUS_UPDATE;
        transition.target_binding.detected_at_ms =
            if transition.predecessor.binding.provider_session_id
                == transition.target_binding.provider_session_id
            {
                DETECTED
            } else {
                TARGET_UPDATE
            };
        transition.target_binding.updated_at_ms = TARGET_UPDATE;
        let authority = set.active().unwrap().clone();
        let case = Self {
            fixture,
            set,
            authority,
        };
        case.set.validate().unwrap();
        case.install(&case.set);
        case
    }

    pub fn transition(&self) -> &PendingBindingTransition {
        self.set.pending_binding_transition.as_ref().unwrap()
    }

    fn install(&self, set: &ClaudeInvocationAuthorities) {
        self.fixture
            .lease
            .write_json(
                &self.fixture.binding_path(),
                &self.transition().predecessor.binding,
            )
            .unwrap();
        self.fixture
            .lease
            .write_json(self.fixture.store.path(), set)
            .unwrap();
    }

    pub fn rejected(&self, changed: ClaudeInvocationAuthorities) {
        // Keep both authorities and proof payload parseable. A rejected store
        // must be detecting their relationship, not malformed scalar input.
        for authority in &changed.authorities {
            record::validate_loaded(authority).unwrap();
        }
        record::validate_loaded(
            &changed
                .pending_binding_transition
                .as_ref()
                .unwrap()
                .predecessor_authority,
        )
        .unwrap();
        let parsed: ClaudeInvocationAuthorities =
            serde_json::from_slice(&serde_json::to_vec(&changed).unwrap()).unwrap();
        assert_eq!(parsed, changed);
        self.install(&changed);
        let before = (
            std::fs::read(self.fixture.store.path()).unwrap(),
            std::fs::read(self.fixture.binding_path()).unwrap(),
        );
        assert!(
            self.fixture
                .store
                .load_in_namespace(&self.fixture.lease)
                .is_err()
        );
        assert!(
            self.fixture
                .store
                .publish_binding_state_in_namespace(
                    &self.fixture.lease,
                    &self.fixture.binding_path(),
                    self.fixture.authentication(),
                    &self.authority,
                    false,
                    |_| Ok(()),
                )
                .is_err()
        );
        assert_eq!(
            (
                std::fs::read(self.fixture.store.path()).unwrap(),
                std::fs::read(self.fixture.binding_path()).unwrap()
            ),
            before
        );
    }

    pub fn commit(&self, expected: &AgentBinding) {
        self.install(&self.set);
        self.fixture
            .store
            .load_in_namespace(&self.fixture.lease)
            .unwrap()
            .unwrap();
        let (_, published) = self
            .fixture
            .store
            .publish_binding_state_in_namespace(
                &self.fixture.lease,
                &self.fixture.binding_path(),
                self.fixture.authentication(),
                &self.authority,
                false,
                |_| Ok(()),
            )
            .unwrap();
        assert_eq!(&published, expected);
        let durable: AgentBinding = self
            .fixture
            .lease
            .read_json(&self.fixture.binding_path())
            .unwrap();
        assert_eq!(durable, published);
    }
}

pub fn state_proof(transition: &PendingBindingTransition) -> bool {
    matches!(
        transition.proof,
        BindingTransitionProof::StateMutation { .. }
    )
}
