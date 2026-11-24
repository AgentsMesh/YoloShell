use super::test_support::{
    CODEX_VERSION, FIRST_POST_START_SEQUENCE, TURN, envelope, identity, started,
};
use super::*;

#[test]
fn snapshot_restore_preserves_the_exact_stop_candidate() {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    started(&mut reducer, &expected);
    let mut stop = envelope(
        &expected,
        CODEX_VERSION,
        FIRST_POST_START_SEQUENCE,
        EventKind::Stop,
    );
    stop.turn_id = Some(TURN.to_owned());
    stop.stop_hook_active = Some(false);
    reducer.apply(stop).unwrap();
    let before = reducer.state().clone();
    let revision = reducer.revision();
    let encoded = serde_json::to_value(reducer.snapshot().unwrap()).unwrap();
    let snapshot = serde_json::from_value::<ReducerSnapshot>(encoded).unwrap();

    let restored =
        Reducer::restore(snapshot, expected, HookProviderKind::Codex, CODEX_VERSION).unwrap();

    assert_eq!(restored.state(), &before);
    assert_eq!(restored.state().turn, TurnState::StopCandidate);
    assert!(restored.state().stop_candidate.is_some());
    assert_eq!(restored.revision(), revision);
}
