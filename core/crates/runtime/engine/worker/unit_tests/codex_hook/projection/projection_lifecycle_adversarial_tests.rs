use super::binding::{self, ProjectionMode};
use super::cursor::{CursorDecision, ProjectionCursor, ProjectionPhase};
use super::forwarder::{ProjectionForwarders, ProjectionInput};
use super::lifecycle_test_storage::{PersistedProjection, assert_explicit_resume};
use super::route::ProjectionRoute;
use super::test_support::{ProjectionFixture, registry};
use crate::agent::hooks::{ProcessState, TurnState};
use crate::agent::{AgentRecoveryAvailability, AgentState};
use std::sync::Arc;
use std::time::Duration;

const FORWARD_TIMEOUT: Duration = Duration::from_secs(1);
const TEST_CHANNEL_CAPACITY: usize = 2;
const COALESCED_SEQUENCE_ADVANCE: u64 = 2;
const COALESCED_REVISION_ADVANCE: u64 = 3;
const CLOSED_INPUT_EXPECTED: &str = "retired Codex projection did not close";
const ACTIVE_INPUT_EXPECTED: &str = "Codex projection did not forward active lifecycle authority";

#[tokio::test]
async fn confirmed_exit_survives_retirement_worker_replacement_and_explicit_resume() {
    let mut fixture = ProjectionFixture::new();
    fixture.start();
    assert_eq!(fixture.prompt().turn, TurnState::Running);
    let committed = fixture.committed.take().unwrap();
    let entry = Arc::clone(&committed.entry);
    let route = ProjectionRoute::capture(&committed);
    let (sender, mut receiver) = tokio::sync::mpsc::channel(TEST_CHANNEL_CAPACITY);
    let mut forwarders = ProjectionForwarders::new();
    forwarders.install(committed, sender);
    let (initial_route, initial) = active(receive(&mut receiver).await);
    let mut cursor = ProjectionCursor::default();
    let initial_version =
        ProjectionCursor::lifecycle_version(&initial_route, &initial, initial.authority_revision());
    cursor.advance(initial_version, ProjectionPhase::Active);
    assert!(!cursor.preserves_closed(&initial_route));

    fixture.pre_compact();
    fixture.finish();
    fixture.confirm_exit();
    let (exit_route, exited) = active(receive(&mut receiver).await);
    assert_eq!(exited.process, ProcessState::Exited);
    assert_eq!(
        exited.last_sequence.0,
        initial.last_sequence.0 + COALESCED_SEQUENCE_ADVANCE
    );
    assert_eq!(
        exited.authority_revision(),
        initial.authority_revision() + COALESCED_REVISION_ADVANCE
    );
    let exit_version =
        ProjectionCursor::lifecycle_version(&exit_route, &exited, exited.authority_revision());
    assert_eq!(
        cursor.classify(exit_version, ProjectionPhase::ConfirmedExit),
        CursorDecision::Apply
    );
    cursor.advance(exit_version, ProjectionPhase::ConfirmedExit);
    let projected = binding::project(
        &route,
        &exited,
        exited.authority_revision(),
        ProjectionMode::Active,
        None,
        &registry(),
    )
    .unwrap();
    let persisted = PersistedProjection::capture(&projected);

    super::super::router::tests::support::ready_retirement(fixture.router.begin_retirement(&entry))
        .commit()
        .unwrap();
    drop(entry);
    drop(fixture.authority.take());
    let ProjectionInput::Closed { route: closed } = receive(&mut receiver).await else {
        panic!("{CLOSED_INPUT_EXPECTED}");
    };
    assert!(cursor.preserves_closed(&closed));
    let recovered = persisted.load();
    assert_eq!(recovered.state, AgentState::Exited);
    assert_eq!(
        recovered.recovery_availability,
        AgentRecoveryAvailability::Available
    );
    assert_explicit_resume(&recovered);
}

async fn receive(receiver: &mut tokio::sync::mpsc::Receiver<ProjectionInput>) -> ProjectionInput {
    tokio::time::timeout(FORWARD_TIMEOUT, receiver.recv())
        .await
        .unwrap()
        .unwrap()
}

fn active(
    input: ProjectionInput,
) -> (
    ProjectionRoute,
    super::super::publication::CodexLifecycleView,
) {
    let ProjectionInput::View {
        route,
        view: super::super::publication::CodexAuthorityView::Active(lifecycle),
    } = input
    else {
        panic!("{ACTIVE_INPUT_EXPECTED}");
    };
    (route, lifecycle)
}
