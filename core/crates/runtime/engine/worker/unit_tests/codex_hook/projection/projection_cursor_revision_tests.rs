use super::*;
use crate::agent::codex::authority::CodexAuthorityVersion;
use crate::agent::hooks::Generation;
use uuid::Uuid;

const ROUTE_REVISION: u64 = 7;
const EVENT_SEQUENCE: u64 = 11;
const NEXT_EVENT_SEQUENCE: u64 = EVENT_SEQUENCE + 1;
const COALESCED_EVENT_SEQUENCE: u64 = NEXT_EVENT_SEQUENCE + 1;
const AUTHORITY_REVISION: u64 = 21;
const NEXT_AUTHORITY_REVISION: u64 = AUTHORITY_REVISION + 1;
const COALESCED_AUTHORITY_REVISION: u64 = NEXT_AUTHORITY_REVISION + 2;
const INVOCATION: u128 = 0x10000000000000000000000000000001;

#[test]
fn coalesced_full_state_revision_is_applied_without_a_synthetic_gap() {
    let mut cursor = ProjectionCursor::default();
    cursor.advance(
        version(EVENT_SEQUENCE, AUTHORITY_REVISION),
        ProjectionPhase::Active,
    );

    assert_eq!(
        cursor.classify(
            version(COALESCED_EVENT_SEQUENCE, COALESCED_AUTHORITY_REVISION,),
            ProjectionPhase::Active,
        ),
        CursorDecision::Apply
    );
}

#[test]
fn process_exit_revision_applies_when_the_event_sequence_is_unchanged() {
    let mut cursor = ProjectionCursor::default();
    cursor.advance(
        version(EVENT_SEQUENCE, AUTHORITY_REVISION),
        ProjectionPhase::Active,
    );

    assert_eq!(
        cursor.classify(
            version(EVENT_SEQUENCE, NEXT_AUTHORITY_REVISION),
            ProjectionPhase::ConfirmedExit,
        ),
        CursorDecision::Apply
    );
}

#[test]
fn same_authority_revision_with_different_sequence_fails_closed() {
    let mut cursor = ProjectionCursor::default();
    cursor.advance(
        version(EVENT_SEQUENCE, AUTHORITY_REVISION),
        ProjectionPhase::Active,
    );

    assert_eq!(
        cursor.classify(
            version(NEXT_EVENT_SEQUENCE, AUTHORITY_REVISION),
            ProjectionPhase::Active,
        ),
        CursorDecision::Conflict
    );
}

fn version(sequence: u64, authority_revision: u64) -> CodexAuthorityVersion {
    CodexAuthorityVersion {
        route_revision: ROUTE_REVISION,
        invocation_id: Uuid::from_u128(INVOCATION),
        generation: Generation::INITIAL,
        authority_revision,
        sequence,
    }
}
