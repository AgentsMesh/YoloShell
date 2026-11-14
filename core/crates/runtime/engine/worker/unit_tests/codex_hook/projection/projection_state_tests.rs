use super::binding::{self, ProjectionMode};
use super::test_support::{
    FIFTH_SEQUENCE, FOURTH_SEQUENCE, ProjectionFixture, THIRD_SEQUENCE, registry, submit_event,
};
use crate::agent::hooks::{AttentionState, EventKind, HookMetadata, TurnState};
use crate::agent::{AgentRecoveryAvailability, AgentState};

const AUTHORITY_REVISION: u64 = 1;

#[test]
fn permission_candidate_preserves_identity_and_execution_clears_it() {
    let fixture = ProjectionFixture::new();
    let route = fixture.route();
    fixture.start();
    fixture.prompt();
    let permission = event(&fixture, THIRD_SEQUENCE, EventKind::PermissionRequest, None);
    assert_eq!(permission.attention, AttentionState::PermissionCandidate);
    assert_projection(&route, &permission, AgentState::WaitingForPermission);

    let running = event(&fixture, FOURTH_SEQUENCE, EventKind::PreToolUse, None);
    assert_eq!(running.attention, AttentionState::None);
    assert_projection(&route, &running, AgentState::Running);
}

#[test]
fn initial_and_reentrant_stop_candidates_do_not_confirm_completion() {
    for continued in [false, true] {
        let fixture = ProjectionFixture::new();
        let route = fixture.route();
        fixture.start();
        fixture.prompt();
        let stopped = event(&fixture, THIRD_SEQUENCE, EventKind::Stop, Some(continued));
        assert_eq!(stopped.turn, TurnState::StopCandidate);
        assert_projection(&route, &stopped, AgentState::Active);
    }
}

#[test]
fn compaction_does_not_promote_a_pending_permission_candidate() {
    let fixture = ProjectionFixture::new();
    let route = fixture.route();
    fixture.start();
    fixture.prompt();
    event(&fixture, THIRD_SEQUENCE, EventKind::PermissionRequest, None);
    let compacting = event(&fixture, FOURTH_SEQUENCE, EventKind::PreCompact, None);
    assert_eq!(compacting.turn, TurnState::Compacting);
    assert_projection(&route, &compacting, AgentState::WaitingForPermission);

    let running = event(&fixture, FIFTH_SEQUENCE, EventKind::PreToolUse, None);
    assert_projection(&route, &running, AgentState::Running);
}

fn event(
    fixture: &ProjectionFixture,
    sequence: u64,
    event: EventKind,
    stop_hook_active: Option<bool>,
) -> crate::agent::hooks::NormalizedLifecycle {
    submit_event(
        fixture.authority.as_ref().unwrap(),
        sequence,
        event,
        HookMetadata::default(),
        stop_hook_active,
    )
}

fn assert_projection(
    route: &super::route::ProjectionRoute,
    lifecycle: &crate::agent::hooks::NormalizedLifecycle,
    expected: AgentState,
) {
    let projected = binding::project(
        route,
        lifecycle,
        AUTHORITY_REVISION,
        ProjectionMode::Active,
        None,
        &registry(),
    )
    .unwrap();
    assert_eq!(projected.binding.state, expected);
    assert_eq!(
        projected.binding.recovery_availability,
        AgentRecoveryAvailability::Available
    );
}
