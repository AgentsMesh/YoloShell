use super::*;
use crate::claude_hooks::MAX_CLAUDE_FAILED_HANDSHAKE_AUDITS;

#[test]
fn persisted_invocations_and_claims_require_unambiguous_nonzero_identities() {
    let fixture = Fixture::new();
    let cases: &[Case] = &[
        (stringify!(nil_invocation), |state| {
            state.invocations[FIRST_QUEUE].credentials.invocation_id = Uuid::nil();
        }),
        (stringify!(duplicate_invocation), |state| {
            state
                .invocations
                .push_back(state.invocations[FIRST_QUEUE].clone());
        }),
        (stringify!(agent_pid), |state| {
            state.invocations[FIRST_QUEUE].agent.pid = u32::default();
        }),
        (stringify!(agent_start_time), |state| {
            state.invocations[FIRST_QUEUE].agent.start_time = u64::default();
        }),
        (stringify!(claim_pid), |state| {
            state.invocations[FIRST_QUEUE].claim.as_mut().unwrap().pid = u32::default();
        }),
        (stringify!(claim_start_time), |state| {
            state.invocations[FIRST_QUEUE]
                .claim
                .as_mut()
                .unwrap()
                .start_time = u64::default();
        }),
    ];
    fixture.corrupt_cases(&fixture.snapshot(), cases);
}

#[test]
fn malformed_retired_handshake_audits_cannot_be_silently_pruned_or_rewritten() {
    let fixture = Fixture::new();
    fixture.add_failed_handshake();
    let original = fixture.snapshot();
    let cases: &[Case] = &[
        (stringify!(audit_invocation), |state| {
            state.failed_handshakes[FIRST_AUDIT].invocation_id = Uuid::nil();
        }),
        (stringify!(audit_pid), |state| {
            state.failed_handshakes[FIRST_AUDIT].agent.pid = u32::default();
        }),
        (stringify!(audit_start_time), |state| {
            state.failed_handshakes[FIRST_AUDIT].agent.start_time = u64::default();
        }),
        (stringify!(audit_first_sequence), |state| {
            state.failed_handshakes[FIRST_AUDIT].first_sequence = u64::default();
        }),
        (stringify!(audit_sequence_order), |state| {
            let audit = &mut state.failed_handshakes[FIRST_AUDIT];
            audit.last_sequence = audit.first_sequence - STEP;
        }),
        (stringify!(audit_event_count), |state| {
            state.failed_handshakes[FIRST_AUDIT].event_count = usize::default();
        }),
        (stringify!(oversized_persisted_audit), |state| {
            let audit = state.failed_handshakes[FIRST_AUDIT].clone();
            state
                .failed_handshakes
                .resize(MAX_CLAUDE_FAILED_HANDSHAKE_AUDITS + COUNT_STEP, audit);
        }),
    ];
    fixture.corrupt_cases(&original, cases);
}
