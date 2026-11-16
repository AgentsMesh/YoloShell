use super::binding::{self, ProjectionMode};
use super::test_support::{ProjectionFixture, registry};
use crate::agent::codex::authority::CodexAuthorityStatus;
use crate::agent::hooks::{
    Generation, InstrumentationState, ProcessState, HookProviderKind, TurnState,
};
use crate::agent::{AgentRecoveryAvailability, AgentState};

const TEST_AUTHORITY_REVISION: u64 = 1;
const IDENTITY_INCREMENT: u64 = 1;
const REVISION_INCREMENT: u64 = 1;
const VERSION_INCREMENT: u16 = 1;
const MISSING_TRANSCRIPT_FILE: &str = "missing-stop-transcript.jsonl";

#[test]
fn authoritative_stop_candidate_preserves_exact_recovery_evidence() {
    let fixture = ProjectionFixture::new();
    let route = fixture.route();
    fixture.start();
    fixture.prompt();
    let stopped = fixture.stop();

    let projected = binding::project(
        &route,
        &stopped,
        TEST_AUTHORITY_REVISION,
        ProjectionMode::Active,
        None,
        &registry(),
    )
    .unwrap();
    let provider = fixture.authority.as_ref().unwrap().identity.process;
    assert_eq!(stopped.turn, TurnState::StopCandidate);
    assert!(stopped.stop_candidate.is_some());
    assert_eq!(projected.status, CodexAuthorityStatus::Active);
    assert_eq!(projected.binding.state, AgentState::Active);
    assert_eq!(
        projected.binding.recovery_availability,
        AgentRecoveryAvailability::Available
    );
    assert_eq!(projected.provider, provider);
    assert_eq!(projected.binding.process_pid, Some(provider.pid));
    assert_eq!(
        projected.binding.process_start_time,
        Some(provider.start_time)
    );
}

#[test]
fn incomplete_stop_lifecycles_remain_unavailable() {
    let fixture = ProjectionFixture::new();
    let route = fixture.route();
    fixture.start();
    fixture.prompt();
    let stopped = fixture.stop();

    for lifecycle in [
        without_stop_candidate(&stopped),
        with_instrumentation(&stopped, InstrumentationState::Pending),
        with_unconfirmed_exit(&stopped),
    ] {
        let projected = binding::project(
            &route,
            &lifecycle,
            TEST_AUTHORITY_REVISION,
            ProjectionMode::Active,
            None,
            &registry(),
        )
        .unwrap();
        assert_eq!(projected.status, CodexAuthorityStatus::Unavailable);
        assert_eq!(projected.binding.state, AgentState::Unknown);
        assert_eq!(
            projected.binding.recovery_availability,
            AgentRecoveryAvailability::Unavailable
        );
    }
}

#[test]
fn mismatched_stop_authority_evidence_is_rejected() {
    let fixture = ProjectionFixture::new();
    let route = fixture.route();
    fixture.start();
    fixture.prompt();
    let stopped = fixture.stop();

    let mut wrong_process = stopped.clone();
    wrong_process.expected_identity.process.start_time += IDENTITY_INCREMENT;
    let mut wrong_generation = stopped.clone();
    wrong_generation.generation = Generation(stopped.generation.0 + IDENTITY_INCREMENT);
    let mut wrong_provider = stopped.clone();
    wrong_provider.provider = HookProviderKind::Claude;
    let mut wrong_version = stopped.clone();
    wrong_version.provider_version.patch += VERSION_INCREMENT;
    let mut missing_transcript = stopped.clone();
    missing_transcript.current_transcript_path = stopped
        .current_transcript_path
        .as_ref()
        .map(|path| path.with_file_name(MISSING_TRANSCRIPT_FILE));

    for lifecycle in [
        wrong_process,
        wrong_generation,
        wrong_provider,
        wrong_version,
        missing_transcript,
    ] {
        assert!(
            binding::project(
                &route,
                &lifecycle,
                TEST_AUTHORITY_REVISION,
                ProjectionMode::Active,
                None,
                &registry(),
            )
            .is_none()
        );
    }

    let mut stale_route = route.clone();
    stale_route.revision += REVISION_INCREMENT;
    for (candidate_route, revision) in [
        (&stale_route, TEST_AUTHORITY_REVISION),
        (&route, u64::default()),
        (&route, u64::MAX),
    ] {
        assert!(
            binding::project(
                candidate_route,
                &stopped,
                revision,
                ProjectionMode::Active,
                None,
                &registry(),
            )
            .is_none()
        );
    }
}

fn without_stop_candidate(
    lifecycle: &crate::agent::hooks::NormalizedLifecycle,
) -> crate::agent::hooks::NormalizedLifecycle {
    let mut lifecycle = lifecycle.clone();
    lifecycle.stop_candidate = None;
    lifecycle
}

fn with_instrumentation(
    lifecycle: &crate::agent::hooks::NormalizedLifecycle,
    instrumentation: InstrumentationState,
) -> crate::agent::hooks::NormalizedLifecycle {
    let mut lifecycle = lifecycle.clone();
    lifecycle.instrumentation = instrumentation;
    lifecycle
}

fn with_unconfirmed_exit(
    lifecycle: &crate::agent::hooks::NormalizedLifecycle,
) -> crate::agent::hooks::NormalizedLifecycle {
    let mut lifecycle = lifecycle.clone();
    lifecycle.process = ProcessState::Exited;
    lifecycle.final_hook_seen = false;
    lifecycle
}
