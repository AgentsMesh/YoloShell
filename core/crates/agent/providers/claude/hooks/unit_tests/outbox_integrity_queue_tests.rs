use super::mutations::{credentials, sequence};
use super::*;
use crate::claude_hooks::{ClaudeHookOperation, ClaudeInvocationMode};

#[test]
fn persisted_sequence_damage_cannot_deliver_or_advance_the_outbox() {
    let fixture = Fixture::new();
    let cases: &[Case] = &[
        (stringify!(duplicate_sequence), |state| {
            let queue = &mut state.invocations[FIRST_QUEUE];
            let first = sequence(&mut queue.pending[SESSION_START_INDEX].request);
            let first = *first;
            *sequence(&mut queue.pending[CWD_CHANGED_INDEX].request) = first;
        }),
        (stringify!(sequence_gap), |state| {
            *sequence(&mut state.invocations[FIRST_QUEUE].pending[CWD_CHANGED_INDEX].request) +=
                STEP;
        }),
        (stringify!(reordered_events), |state| {
            state.invocations[FIRST_QUEUE]
                .pending
                .swap(SESSION_START_INDEX, CWD_CHANGED_INDEX);
        }),
        (stringify!(stale_acknowledgement), |state| {
            state.invocations[FIRST_QUEUE].acknowledged_sequence += STEP;
        }),
        (stringify!(wrong_next_sequence), |state| {
            state.invocations[FIRST_QUEUE].next_sequence += STEP;
        }),
        (stringify!(exhausted_acknowledgement), |state| {
            state.invocations[FIRST_QUEUE].acknowledged_sequence = u64::MAX;
        }),
    ];
    fixture.corrupt_cases(&fixture.snapshot(), cases);
}

#[test]
fn persisted_requests_cannot_borrow_the_queues_credentials_or_schema() {
    let fixture = Fixture::new();
    let cases: &[Case] = &[
        (stringify!(request_schema_version), |state| {
            state.invocations[FIRST_QUEUE].pending[STATE_CHANGED_INDEX]
                .request
                .schema_version += SCHEMA_VERSION_STEP;
        }),
        (stringify!(request_terminal_id), |state| {
            credentials(state).terminal_id = Uuid::new_v4();
        }),
        (stringify!(request_runtime_epoch), |state| {
            credentials(state).runtime_epoch = Uuid::new_v4();
        }),
        (stringify!(request_invocation_id), |state| {
            credentials(state).invocation_id = Uuid::new_v4();
        }),
        (stringify!(request_hook_token), |state| {
            credentials(state).hook_token = Uuid::new_v4();
        }),
        (stringify!(request_helper_capability), |state| {
            credentials(state).helper_capability = Uuid::new_v4();
        }),
    ];
    fixture.corrupt_cases(&fixture.snapshot(), cases);
}

#[test]
fn persisted_event_kinds_require_their_exact_transcript_evidence_shape() {
    let fixture = Fixture::new();
    let original = fixture.snapshot();
    let cases: &[Case] = &[
        (stringify!(session_start_without_identity), |state| {
            state.invocations[FIRST_QUEUE].pending[SESSION_START_INDEX].transcript_identity = None;
        }),
        (stringify!(cwd_changed_without_identity), |state| {
            state.invocations[FIRST_QUEUE].pending[CWD_CHANGED_INDEX].transcript_identity = None;
        }),
        (stringify!(state_changed_with_identity), |state| {
            let queue = &mut state.invocations[FIRST_QUEUE];
            queue.pending[STATE_CHANGED_INDEX].transcript_identity =
                queue.pending[SESSION_START_INDEX].transcript_identity;
        }),
        (stringify!(session_end_with_identity), |state| {
            let queue = &mut state.invocations[FIRST_QUEUE];
            queue.pending[SESSION_END_INDEX].transcript_identity =
                queue.pending[SESSION_START_INDEX].transcript_identity;
        }),
        (stringify!(begin_in_lifecycle_queue), |state| {
            let queue = &mut state.invocations[FIRST_QUEUE];
            let ClaudeHookOperation::Lifecycle {
                event:
                    ClaudeLifecycleEvent::SessionStart {
                        provider_session_id,
                        cwd,
                        ..
                    },
                ..
            } = &queue.pending[SESSION_START_INDEX].request.operation
            else {
                unreachable!()
            };
            queue.pending[STATE_CHANGED_INDEX].request.operation = ClaudeHookOperation::Begin {
                provider_session_id: provider_session_id.clone(),
                discover_session: false,
                mode: ClaudeInvocationMode::Fresh,
                cwd: cwd.clone(),
                data_home: cwd.clone(),
            };
        }),
    ];
    fixture.corrupt_cases(&original, cases);
}
