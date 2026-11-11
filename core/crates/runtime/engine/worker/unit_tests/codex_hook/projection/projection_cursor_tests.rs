use super::*;
use crate::agent::hooks::Generation;
use uuid::Uuid;

const ROUTE: u64 = 7;
const NEXT_ROUTE: u64 = ROUTE + 1;
const SEQUENCE: u64 = 11;
const NEXT_SEQUENCE: u64 = SEQUENCE + 1;
const AUTHORITY_REVISION: u64 = 21;
const INVOCATION: u128 = 0x10000000000000000000000000000001;
const OTHER_INVOCATION: u128 = INVOCATION + 1;

#[test]
fn first_recovered_sequence_is_admitted_without_a_synthetic_gap() {
    let cursor = ProjectionCursor::default();
    assert_eq!(
        cursor.classify(
            version(ROUTE, INVOCATION, Generation::INITIAL, SEQUENCE),
            phase()
        ),
        CursorDecision::Apply
    );
}

#[test]
fn exact_phase_and_version_replay_is_idempotent() {
    let mut cursor = ProjectionCursor::default();
    let current = version(ROUTE, INVOCATION, Generation::INITIAL, SEQUENCE);
    cursor.advance(current, phase());
    assert_eq!(cursor.classify(current, phase()), CursorDecision::Replay);
}

#[test]
fn same_version_only_moves_toward_terminal_authority() {
    let mut cursor = ProjectionCursor::default();
    let current = version(ROUTE, INVOCATION, Generation::INITIAL, SEQUENCE);
    cursor.advance(current, ProjectionPhase::Active);
    assert_eq!(
        cursor.classify(current, ProjectionPhase::Exhausted),
        CursorDecision::Apply
    );
    cursor.advance(current, ProjectionPhase::Exhausted);
    assert_eq!(
        cursor.classify(current, ProjectionPhase::Terminal),
        CursorDecision::Apply
    );
    cursor.advance(current, ProjectionPhase::Terminal);
    assert_eq!(
        cursor.classify(current, ProjectionPhase::Active),
        CursorDecision::Stale
    );
}

#[test]
fn successor_generation_and_route_replace_without_a_sequence_gap() {
    let mut cursor = ProjectionCursor::default();
    cursor.advance(
        version(ROUTE, INVOCATION, Generation::INITIAL, SEQUENCE),
        phase(),
    );
    let successor = Generation(Generation::INITIAL.0 + 1);
    assert_eq!(
        cursor.classify(version(ROUTE, INVOCATION, successor, SEQUENCE), phase()),
        CursorDecision::Apply
    );
    assert_eq!(
        cursor.classify(
            version(NEXT_ROUTE, OTHER_INVOCATION, Generation::INITIAL, SEQUENCE),
            phase()
        ),
        CursorDecision::Apply
    );
}

#[test]
fn stale_route_generation_and_substituted_invocation_are_rejected() {
    let mut cursor = ProjectionCursor::default();
    let successor = Generation(Generation::INITIAL.0 + 1);
    cursor.advance(
        version(NEXT_ROUTE, OTHER_INVOCATION, successor, NEXT_SEQUENCE),
        phase(),
    );
    assert_eq!(
        cursor.classify(
            version(ROUTE, INVOCATION, successor, NEXT_SEQUENCE),
            phase()
        ),
        CursorDecision::Stale
    );
    assert_eq!(
        cursor.classify(
            version(
                NEXT_ROUTE,
                OTHER_INVOCATION,
                Generation::INITIAL,
                NEXT_SEQUENCE
            ),
            phase()
        ),
        CursorDecision::Stale
    );
    assert_eq!(
        cursor.classify(
            version(NEXT_ROUTE, INVOCATION, successor, NEXT_SEQUENCE),
            phase()
        ),
        CursorDecision::Conflict
    );
}

fn phase() -> ProjectionPhase {
    ProjectionPhase::Active
}

fn version(
    route_revision: u64,
    invocation: u128,
    generation: Generation,
    sequence: u64,
) -> CodexAuthorityVersion {
    version_at(
        route_revision,
        invocation,
        generation,
        sequence,
        AUTHORITY_REVISION,
    )
}

fn version_at(
    route_revision: u64,
    invocation: u128,
    generation: Generation,
    sequence: u64,
    authority_revision: u64,
) -> CodexAuthorityVersion {
    CodexAuthorityVersion {
        route_revision,
        invocation_id: Uuid::from_u128(invocation),
        generation,
        authority_revision,
        sequence,
    }
}
