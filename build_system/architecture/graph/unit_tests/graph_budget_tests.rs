use super::*;

#[test]
fn graph_state_budget_has_an_absolute_upper_bound() {
    assert_eq!(
        states(usize::MAX, usize::MAX, usize::MAX),
        MAXIMUM_GRAPH_STATES,
    );
}
