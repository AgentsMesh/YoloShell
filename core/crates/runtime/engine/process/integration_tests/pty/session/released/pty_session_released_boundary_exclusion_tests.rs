use super::*;

#[test]
fn controller_boundary_is_removed_before_scheduling_and_state_reconciliation() {
    let blocked = candidate(FIRST_PID, FIRST_GENERATION, true);
    let retained = candidate(SECOND_PID, SECOND_GENERATION, true);
    let excluded = HashSet::from([blocked.live]);
    let current = universe::exclude_candidates(candidates([blocked, retained]), &excluded);
    let mut state = ReleasedScanState::default();
    state.matched.insert(blocked.live, blocked);
    state.matched.insert(retained.live, retained);
    state.holder_hints.extend([blocked.live, retained.live]);

    let state = state.excluding(&excluded);

    assert_eq!(current.as_ref(), [retained]);
    assert!(!state.matched.contains_key(&blocked.live));
    assert!(state.matched.contains_key(&retained.live));
    assert!(!state.holder_hints.contains(&blocked.live));
    assert!(state.holder_hints.contains(&retained.live));
}

#[test]
fn controller_boundary_exclusion_is_scoped_to_the_exact_generation() {
    let blocked = candidate(FIRST_PID, FIRST_GENERATION, true);
    let replacement = candidate(FIRST_PID, SECOND_GENERATION, true);
    let excluded = HashSet::from([blocked.live]);
    let current = universe::exclude_candidates(candidates([blocked, replacement]), &excluded);
    let mut state = ReleasedScanState::default();
    state.matched.insert(blocked.live, blocked);
    state.matched.insert(replacement.live, replacement);

    let state = state.excluding(&excluded);

    assert_eq!(current.as_ref(), [replacement]);
    assert!(!state.matched.contains_key(&blocked.live));
    assert!(state.matched.contains_key(&replacement.live));
}
