use super::support::*;
use crate::agent::hooks::{
    ApplyDisposition, InstrumentationState, ProcessExitDisposition, ProcessIdentity, ProcessState,
    TurnState,
};
use std::process::Command;
use std::sync::Arc;

const EXIT_RECOVERY_SLEEP_EXECUTABLE: &str = "/bin/sleep";
const EXIT_RECOVERY_SLEEP_DURATION: &str = "30";

#[test]
fn process_exit_persists_genesis_authority_without_an_outbox_owner() {
    let fixture = Fixture::new();
    let authority = fixture.runtime.subscribe_authority();

    let publication = fixture
        .runtime
        .observe_process_exit(observed_process(&fixture))
        .unwrap();
    let committed = fixture.runtime.state().unwrap().unwrap();
    let (runtime, report) = fixture.restart_runtime();

    assert_eq!(publication.disposition, ProcessExitDisposition::Unconfirmed);
    assert_eq!(publication.replay, Default::default());
    assert_eq!(committed.process, ProcessState::Exited);
    assert_eq!(report, Default::default());
    assert_eq!(runtime.state().unwrap().unwrap(), committed);
    assert!(matches!(
        authority.borrow().clone(),
        super::super::publication::CodexAuthorityView::Active(state)
            if state.process == ProcessState::Exited
    ));
}

#[test]
fn exited_authority_restores_after_the_exact_agent_process_is_gone() {
    let mut child = crate::process::spawn_waitable_test_child(
        Command::new(EXIT_RECOVERY_SLEEP_EXECUTABLE).arg(EXIT_RECOVERY_SLEEP_DURATION),
    );
    let process = crate::process::ProcessIdentity::capture(child.id()).unwrap();
    let fixture = Fixture::for_process(process);
    child.kill().unwrap();
    child.wait().unwrap();

    assert!(!process.matches());
    assert!(fixture.try_restart_runtime().is_err());
    fixture
        .runtime
        .observe_process_exit(ProcessIdentity::new(process.pid, process.start_time))
        .unwrap();

    let (runtime, report) = fixture.restart_runtime();
    assert_eq!(report, Default::default());
    assert_eq!(
        runtime.state().unwrap().unwrap().process,
        ProcessState::Exited
    );
}

#[test]
fn process_exit_reconciles_pending_hooks_before_persisting_exit() {
    let fixture = Fixture::new();
    fixture.enqueue(fixture.start(UNASSIGNED_EVENT_SEQUENCE));

    let publication = fixture
        .runtime
        .observe_process_exit(observed_process(&fixture))
        .unwrap();
    let state = fixture.runtime.state().unwrap().unwrap();

    assert_eq!(publication.replay.applied, SINGLE_EVENT_COUNT);
    assert_eq!(state.last_sequence.0, FIRST_EVENT_SEQUENCE);
    assert_eq!(state.process, ProcessState::Exited);
    assert_eq!(state.turn, TurnState::Unknown);
    assert_eq!(state.instrumentation, InstrumentationState::Unavailable);
}

#[test]
fn mismatched_exit_identity_persists_revoked_authority_before_rejection() {
    let fixture = Fixture::new();
    let observed = ProcessIdentity::new(
        fixture.process.pid,
        fixture
            .process
            .start_time
            .checked_add(INVALID_PROCESS_START_DELTA)
            .unwrap(),
    );

    let rejection = fixture.runtime.observe_process_exit(observed).unwrap_err();
    let state = fixture.runtime.state().unwrap().unwrap();

    assert!(matches!(
        rejection,
        super::super::error::CodexHookRejection::Violation(
            crate::agent::hooks::HookViolation::ProcessMismatch
        )
    ));
    assert!(state.revoked);
    assert!(fixture.runtime.snapshot_path().exists());
}

#[test]
fn concurrent_ingest_and_exit_are_serialized_behind_pending_reconciliation() {
    let fixture = Fixture::new();
    let event = fixture.enqueue(fixture.start(UNASSIGNED_EVENT_SEQUENCE));
    let ingest_runtime = Arc::clone(&fixture.runtime);
    let exit_runtime = Arc::clone(&fixture.runtime);
    let peer_pid = fixture.process.pid;
    let observed = observed_process(&fixture);

    let ingest = std::thread::spawn(move || ingest_runtime.ingest(event, peer_pid));
    let exit = std::thread::spawn(move || exit_runtime.observe_process_exit(observed));
    let ingestion = ingest.join().unwrap().unwrap();
    let publication = exit.join().unwrap().unwrap();
    let state = fixture.runtime.state().unwrap().unwrap();

    assert!(matches!(
        ingestion.disposition,
        ApplyDisposition::Applied | ApplyDisposition::Replay
    ));
    assert_eq!(publication.disposition, ProcessExitDisposition::Unconfirmed);
    assert_eq!(state.last_sequence.0, FIRST_EVENT_SEQUENCE);
    assert_eq!(state.process, ProcessState::Exited);
}

fn observed_process(fixture: &Fixture) -> ProcessIdentity {
    ProcessIdentity::new(fixture.process.pid, fixture.process.start_time)
}
