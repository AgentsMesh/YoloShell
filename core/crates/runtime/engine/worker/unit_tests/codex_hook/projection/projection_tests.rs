use super::super::publication::CodexAuthorityView;
use super::binding::{self, ProjectionMode};
use super::forwarder::{ProjectionForwarders, ProjectionInput};
use super::test_support::{PROVIDER_SESSION, ProjectionFixture, TRANSCRIPT_FILE, registry};
use crate::agent::codex::authority::CodexAuthorityStatus;
use crate::agent::hooks::{InstrumentationState, ProcessState, HookProviderKind, TurnState};
use crate::agent::{AgentRecoveryAvailability, AgentState};
use std::sync::Arc;
use std::time::Duration;

const FORWARD_TIMEOUT: Duration = Duration::from_secs(1);
const TEST_CHANNEL_CAPACITY: usize = 2;
const TEST_AUTHORITY_REVISION: u64 = 1;

#[test]
fn authoritative_idle_and_running_lifecycles_map_to_recoverable_bindings() {
    let fixture = ProjectionFixture::new();
    let route = fixture.route();
    let idle = fixture.start();
    let waiting = binding::project(
        &route,
        &idle,
        TEST_AUTHORITY_REVISION,
        ProjectionMode::Active,
        None, &registry(),
    )
    .unwrap();
    assert_eq!(waiting.status, CodexAuthorityStatus::Active);
    assert_eq!(waiting.binding.state, AgentState::WaitingForInput);
    assert_eq!(
        waiting.binding.recovery_availability,
        AgentRecoveryAvailability::Available
    );
    assert_eq!(waiting.binding.provider_session_id, PROVIDER_SESSION);
    assert!(waiting.binding.evidence_path.ends_with(TRANSCRIPT_FILE));

    let running = fixture.prompt();
    let projected = binding::project(
        &route,
        &running,
        TEST_AUTHORITY_REVISION,
        ProjectionMode::Active,
        Some(&waiting.binding),
        &registry(),
    )
    .unwrap();
    assert_eq!(projected.binding.state, AgentState::Running);
    assert_eq!(
        projected.binding.detected_at_ms,
        waiting.binding.detected_at_ms
    );
    assert_eq!(
        projected.provider,
        fixture.authority.as_ref().unwrap().identity.process
    );
}

#[test]
fn exhausted_pending_and_unconfirmed_lifecycles_fail_closed() {
    let fixture = ProjectionFixture::new();
    let route = fixture.route();
    let lifecycle = fixture.start();
    let exhausted = binding::project(
        &route,
        &lifecycle,
        TEST_AUTHORITY_REVISION,
        ProjectionMode::Exhausted,
        None, &registry(),
    )
    .unwrap();
    assert_eq!(exhausted.status, CodexAuthorityStatus::Exhausted);
    assert_unavailable(&exhausted.binding);

    let mut pending = lifecycle.clone();
    pending.instrumentation = InstrumentationState::Pending;
    let projected = binding::project(
        &route,
        &pending,
        TEST_AUTHORITY_REVISION,
        ProjectionMode::Active,
        None, &registry(),
    )
    .unwrap();
    assert_eq!(projected.status, CodexAuthorityStatus::Unavailable);
    assert_unavailable(&projected.binding);

    let mut exited = lifecycle;
    exited.process = ProcessState::Exited;
    exited.final_hook_seen = false;
    let projected = binding::project(
        &route,
        &exited,
        TEST_AUTHORITY_REVISION,
        ProjectionMode::Active,
        None, &registry(),
    )
    .unwrap();
    assert_unavailable(&projected.binding);
}

#[test]
fn confirmed_exit_is_recoverable_and_drops_the_live_process_identity() {
    let fixture = ProjectionFixture::new();
    let route = fixture.route();
    let mut lifecycle = fixture.start();
    lifecycle.process = ProcessState::Exited;
    lifecycle.turn = TurnState::Idle;
    lifecycle.final_hook_seen = true;
    let projected = binding::project(
        &route,
        &lifecycle,
        TEST_AUTHORITY_REVISION,
        ProjectionMode::Active,
        None, &registry(),
    )
    .unwrap();
    assert_eq!(projected.binding.state, AgentState::Exited);
    assert_eq!(
        projected.binding.recovery_availability,
        AgentRecoveryAvailability::Available
    );
    assert_eq!(projected.binding.process_pid, None);
    assert_eq!(projected.binding.process_start_time, None);
}

#[test]
fn invalid_provider_or_missing_transcript_cannot_create_a_binding() {
    let fixture = ProjectionFixture::new();
    let route = fixture.route();
    let mut lifecycle = fixture.start();
    lifecycle.provider = HookProviderKind::Claude;
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
    lifecycle.provider = HookProviderKind::Codex;
    lifecycle.current_transcript_path = None;
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

#[tokio::test]
async fn forwarder_releases_runtime_and_reports_channel_close_after_retirement() {
    let mut fixture = ProjectionFixture::new();
    fixture.start();
    let committed = fixture.committed.take().unwrap();
    let entry = Arc::clone(&committed.entry);
    let (sender, mut receiver) = tokio::sync::mpsc::channel(TEST_CHANNEL_CAPACITY);
    let mut forwarders = ProjectionForwarders::new();
    forwarders.install(committed, sender);
    let initial = tokio::time::timeout(FORWARD_TIMEOUT, receiver.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        initial,
        ProjectionInput::View {
            view: CodexAuthorityView::Active(_),
            ..
        }
    ));

    super::super::router::tests::support::ready_retirement(fixture.router.begin_retirement(&entry))
        .commit()
        .unwrap();
    drop(entry);
    drop(fixture.authority.take());
    let closed = tokio::time::timeout(FORWARD_TIMEOUT, receiver.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(closed, ProjectionInput::Closed { .. }));
}

fn assert_unavailable(binding: &crate::agent::AgentBinding) {
    assert_eq!(binding.state, AgentState::Unknown);
    assert_eq!(
        binding.recovery_availability,
        AgentRecoveryAvailability::Unavailable
    );
}
