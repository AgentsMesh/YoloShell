use super::super::{ProcessExitDisposition, SessionEndReason};
use super::support::*;
use crate::agent::{AgentRecoveryAvailability, AgentState};
use crate::process::ProcessIdentity;

#[path = "process_exit_test_support.rs"]
mod exit_support;
use exit_support::ended_fixture;

const PROCESS_EXIT_START_DELTA: u64 = 1;

#[test]
fn final_hook_candidate_and_exact_gone_proof_publish_exited() {
    let (fixture, expected) = ended_fixture(SessionEndReason::Other);
    let _gone = gone_override();
    let disposition = finalize(&fixture, &expected);
    assert_eq!(disposition, ProcessExitDisposition::Confirmed);

    let authority = fixture
        .store
        .load_in_namespace(&fixture.lease)
        .unwrap()
        .unwrap();
    let binding = yolo_agent_binding_io::AgentBindingFile::new(fixture.binding_path())
        .read_in_namespace(&fixture.lease)
        .unwrap()
        .unwrap();
    assert_eq!(authority.state, AgentState::Exited);
    assert!(authority.termination_candidate.is_none());
    assert_eq!(binding.state, AgentState::Exited);
    assert_eq!(
        binding.recovery_availability,
        AgentRecoveryAvailability::Available
    );
    assert!(binding.process_pid.is_none());
    assert!(binding.process_start_time.is_none());
    assert!(authority.matches_binding(&binding));
}

#[test]
fn recovery_boundary_never_becomes_exited_from_process_proof_alone() {
    for reason in [SessionEndReason::Clear, SessionEndReason::Resume] {
        let (fixture, expected) = ended_fixture(reason);
        let _gone = gone_override();
        assert_eq!(
            finalize(&fixture, &expected),
            ProcessExitDisposition::NotCandidate
        );
        let authority = fixture
            .store
            .load_in_namespace(&fixture.lease)
            .unwrap()
            .unwrap();
        assert_eq!(authority.state, AgentState::Unknown);
        assert_eq!(authority.termination_candidate.unwrap().reason, reason);
    }
}

#[test]
fn identity_mismatch_and_unavailable_observation_fail_closed_without_mutation() {
    let (fixture, expected) = ended_fixture(SessionEndReason::Other);
    let authority_before = fixture
        .store
        .load_in_namespace(&fixture.lease)
        .unwrap()
        .unwrap();
    let mismatched = ProcessIdentity {
        pid: PROCESS_PID,
        start_time: PROCESS_START + PROCESS_EXIT_START_DELTA,
    };
    assert!(
        fixture
            .store
            .finalize_process_exit_in_namespace(
                &fixture.lease,
                expected.authentication(),
                &expected,
                mismatched,
            )
            .is_err()
    );
    assert_eq!(
        fixture
            .store
            .load_in_namespace(&fixture.lease)
            .unwrap()
            .unwrap(),
        authority_before
    );

    let _unknown = crate::process::install_test_process_identity_relation(
        PROCESS_PID,
        PROCESS_START,
        crate::process::ProcessIdentityRelation::Unknown,
    );
    assert_eq!(
        finalize(&fixture, &expected),
        ProcessExitDisposition::Unconfirmed
    );
    assert_eq!(
        fixture
            .store
            .load_in_namespace(&fixture.lease)
            .unwrap()
            .unwrap(),
        authority_before
    );
}

#[test]
fn matching_process_and_replay_are_not_treated_as_exits() {
    let (fixture, expected) = ended_fixture(SessionEndReason::Other);
    let _matching = crate::process::install_test_process_identity_relation(
        PROCESS_PID,
        PROCESS_START,
        crate::process::ProcessIdentityRelation::Matching,
    );
    assert_eq!(
        finalize(&fixture, &expected),
        ProcessExitDisposition::Unconfirmed
    );

    let _gone = gone_override();
    assert_eq!(
        finalize(&fixture, &expected),
        ProcessExitDisposition::Confirmed
    );
    let committed = fixture
        .store
        .load_in_namespace(&fixture.lease)
        .unwrap()
        .unwrap();
    assert_eq!(
        finalize(&fixture, &committed),
        ProcessExitDisposition::AlreadyExited
    );
}

fn finalize(
    fixture: &Fixture,
    expected: &super::super::ClaudeGenerationAuthority,
) -> ProcessExitDisposition {
    fixture
        .store
        .finalize_process_exit_in_namespace(
            &fixture.lease,
            expected.authentication(),
            expected,
            ProcessIdentity {
                pid: PROCESS_PID,
                start_time: PROCESS_START,
            },
        )
        .unwrap()
}

fn gone_override() -> crate::process::ProcessIdentityRelationGuard {
    crate::process::install_test_process_identity_relation(
        PROCESS_PID,
        PROCESS_START,
        crate::process::ProcessIdentityRelation::Gone,
    )
}
