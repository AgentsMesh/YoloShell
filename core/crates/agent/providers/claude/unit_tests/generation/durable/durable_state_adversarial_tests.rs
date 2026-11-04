use super::super::SessionStartSource;
use super::support::*;
use crate::agent::claude::generation::adversarial_test_support::{
    PersistenceBoundary, persist_durable_state, persist_durable_state_with_boundary,
};
use crate::agent::{
    AgentBinding, AgentBindingStore, AgentKind, AgentRecoveryAvailability, AgentState,
};

const INJECTED_FAILURE: &str = "injected durable state failure";
const EVIDENCE_MUTATION_SUFFIX: &str = ".changed";

#[test]
fn every_durable_state_has_an_exact_idempotent_publication() {
    for state in [
        AgentState::Running,
        AgentState::WaitingForInput,
        AgentState::Exited,
        AgentState::Unknown,
    ] {
        let fixture = published_fixture();
        let generation_before = publication_generation(&fixture);

        persist_durable_state(&fixture.binding_path(), state).unwrap();
        assert_durable_state(&fixture, state);
        let generation_after = publication_generation(&fixture);
        let expected_generation = if state == AgentState::Running {
            generation_before
        } else {
            generation_before
                .checked_add(super::super::constants::REVISION_INCREMENT)
                .unwrap()
        };
        assert_eq!(generation_after, expected_generation);

        persist_durable_state(&fixture.binding_path(), state).unwrap();
        assert_durable_state(&fixture, state);
        assert_eq!(publication_generation(&fixture), generation_after);
    }
}

#[test]
fn every_persistence_boundary_converges_on_retry() {
    for state in [
        AgentState::WaitingForInput,
        AgentState::Exited,
        AgentState::Unknown,
    ] {
        for boundary in [
            PersistenceBoundary::AuthorityPending,
            PersistenceBoundary::BindingWritten,
        ] {
            assert_boundary_retry(state, boundary);
        }
    }
}

fn assert_boundary_retry(state: AgentState, boundary: PersistenceBoundary) {
    let fixture = published_fixture();
    let generation_before = publication_generation(&fixture);
    let error = persist_durable_state_with_boundary(&fixture.binding_path(), state, |actual| {
        if actual == boundary {
            anyhow::bail!(INJECTED_FAILURE);
        }
        Ok(())
    })
    .unwrap_err();
    assert!(error.to_string().contains(INJECTED_FAILURE));

    persist_durable_state(&fixture.binding_path(), state).unwrap();
    assert_durable_state(&fixture, state);
    assert_eq!(
        publication_generation(&fixture),
        generation_before
            .checked_add(super::super::constants::REVISION_INCREMENT)
            .unwrap()
    );
}

#[test]
fn mismatched_binding_identity_fails_closed_without_mutating_authority() {
    for mutate in [
        mutate_kind as fn(&mut AgentBinding),
        mutate_session,
        mutate_evidence,
    ] {
        let fixture = published_fixture();
        let authority_before = authority(&fixture);
        let generation_before = publication_generation(&fixture);
        let binding_store = AgentBindingStore::new(fixture.binding_path());
        let mut binding = binding_store
            .read_raw_in_namespace(&fixture.lease)
            .unwrap()
            .unwrap();
        mutate(&mut binding);
        fixture
            .lease
            .write_json(&fixture.binding_path(), &binding)
            .unwrap();

        assert!(
            persist_durable_state(&fixture.binding_path(), AgentState::WaitingForInput).is_err()
        );
        assert_eq!(authority(&fixture), authority_before);
        assert_eq!(publication_generation(&fixture), generation_before);
    }
}

fn published_fixture() -> Fixture {
    let fixture = Fixture::new();
    let session_id = fixture.begin.provider_session_id.clone();
    fixture
        .store
        .begin_in_namespace(&fixture.lease, &fixture.begin)
        .unwrap();
    let authority = fixture
        .store
        .apply_in_namespace(
            &fixture.lease,
            fixture.authentication(),
            &fixture.start(&session_id, SessionStartSource::Fresh),
        )
        .unwrap()
        .0;
    AgentBindingStore::new(fixture.binding_path())
        .save_authoritative_in_namespace(
            &fixture.lease,
            &binding(&authority),
            &authority,
            fixture.runtime(),
        )
        .unwrap();
    fixture
}

fn assert_durable_state(fixture: &Fixture, state: AgentState) {
    let authority = authority(fixture);
    let binding = AgentBindingStore::new(fixture.binding_path())
        .read_raw_in_namespace(&fixture.lease)
        .unwrap()
        .unwrap();
    assert_eq!(authority.state, state);
    assert_eq!(binding.state, state);
    if state == AgentState::Unknown {
        assert!(authority.binding_revision.is_none());
        assert_eq!(
            binding.recovery_availability,
            AgentRecoveryAvailability::Unavailable
        );
        assert!(authority.transcript_path.is_none());
    } else {
        assert!(authority.matches_binding(&binding));
        assert_eq!(
            binding.recovery_availability,
            AgentRecoveryAvailability::Available
        );
    }
    let expected_process = state != AgentState::Exited;
    assert_eq!(binding.process_pid.is_some(), expected_process);
    assert_eq!(binding.process_start_time.is_some(), expected_process);
}

fn authority(fixture: &Fixture) -> super::super::ClaudeGenerationAuthority {
    fixture
        .store
        .load_in_namespace(&fixture.lease)
        .unwrap()
        .unwrap()
}

fn publication_generation(fixture: &Fixture) -> u64 {
    fixture
        .store
        .publication_generation_in_namespace(&fixture.lease)
        .unwrap()
}

fn mutate_kind(binding: &mut AgentBinding) {
    binding.kind = AgentKind::Codex;
}

fn mutate_session(binding: &mut AgentBinding) {
    binding.provider_session_id = session_id();
}

fn mutate_evidence(binding: &mut AgentBinding) {
    binding.evidence_path.push_str(EVIDENCE_MUTATION_SUFFIX);
}
