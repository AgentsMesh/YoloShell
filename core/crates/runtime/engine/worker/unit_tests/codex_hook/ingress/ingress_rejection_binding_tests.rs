use super::support::{FIRST_EVENT_SEQUENCE, Fixture, INVALID_PROCESS_START_DELTA};
use crate::agent::hooks::{HookEnvelope, HookViolation, ProcessIdentity, HookProviderKind};
use crate::codex_hooks::{
    CodexDurableRejection, CodexEnqueueDecision, CodexOutbox, CodexOutboxState,
};

#[test]
fn revoked_snapshot_rejects_a_structurally_valid_marker_violation_replacement() {
    let (fixture, outbox, event) = fully_revoked_hook_authority();
    let mut state = outbox.load().unwrap();
    state.durable_rejections_mut_for_test().clear();
    state.durable_rejections_mut_for_test().push_back(
        CodexDurableRejection::capture_for_test(&event, HookViolation::InvalidTopology).unwrap(),
    );
    persist_valid_tamper(&outbox, &fixture, state);

    assert!(fixture.try_restart_runtime().is_err());
}

#[test]
fn revoked_snapshot_rejects_an_envelope_and_marker_replacement() {
    let (fixture, outbox, _) = fully_revoked_hook_authority();
    let mut replacement = fixture.start(FIRST_EVENT_SEQUENCE);
    replacement.event_id = Fixture::other_event_id();
    let marker =
        CodexDurableRejection::capture_for_test(&replacement, HookViolation::InvalidTopology)
            .unwrap();
    let mut state = outbox.load().unwrap();
    state.pending_mut_for_test().clear();
    state.pending_mut_for_test().push_back(replacement);
    state.durable_rejections_mut_for_test().clear();
    state.durable_rejections_mut_for_test().push_back(marker);
    persist_valid_tamper(&outbox, &fixture, state);

    assert!(fixture.try_restart_runtime().is_err());
}

#[test]
fn process_observation_revocation_rejects_a_fabricated_marker() {
    let fixture = Fixture::new();
    let observed = ProcessIdentity::new(
        fixture.process.pid,
        fixture
            .process
            .start_time
            .checked_add(INVALID_PROCESS_START_DELTA)
            .unwrap(),
    );
    assert!(fixture.runtime.observe_process_exit(observed).is_err());
    let mut event = fixture.start(FIRST_EVENT_SEQUENCE);
    event.provider = HookProviderKind::Claude;
    let (outbox, _) = enqueue_rejection(&fixture, event, HookViolation::ProviderMismatch);

    assert!(outbox.load().is_ok());
    assert!(fixture.try_restart_runtime().is_err());
}

fn fully_revoked_hook_authority() -> (Fixture, CodexOutbox, HookEnvelope) {
    let fixture = Fixture::new();
    let event = fixture.prompt(FIRST_EVENT_SEQUENCE);
    let (outbox, event) = enqueue_rejection(&fixture, event, HookViolation::SessionNotStarted);
    let (runtime, report) = fixture.restart_runtime();
    assert_eq!(report, Default::default());
    assert!(runtime.state().unwrap().unwrap().revoked);
    drop(runtime);
    (fixture, outbox, event)
}

fn enqueue_rejection(
    fixture: &Fixture,
    event: HookEnvelope,
    violation: HookViolation,
) -> (CodexOutbox, HookEnvelope) {
    let outbox =
        CodexOutbox::for_identity(fixture.runtime.outbox_path(), &fixture.identity).unwrap();
    let committed = outbox
        .enqueue_admitted_decision(
            event,
            None,
            |_, _| Ok(CodexEnqueueDecision::rejected((), violation)),
            |_, _, _| Ok(()),
        )
        .unwrap();
    (outbox, committed.into_parts().0)
}

fn persist_valid_tamper(outbox: &CodexOutbox, fixture: &Fixture, state: CodexOutboxState) {
    crate::paths::atomic_write_json(outbox.path(), &state).unwrap();
    assert!(CodexOutbox::for_identity(outbox.path(), &fixture.identity).is_ok());
}
