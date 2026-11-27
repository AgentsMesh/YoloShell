use super::ProcessTransitionError;
use std::error::Error;

const TRANSITION_FAILURE_DETAIL: &str = "injected transition failure";
const FIRST_TRANSITION_STATE: u32 = 17;
const SECOND_TRANSITION_STATE: u32 = 23;

#[test]
fn transition_error_preserves_state_source_and_formatting() {
    let failure = ProcessTransitionError::new(
        FIRST_TRANSITION_STATE,
        anyhow::anyhow!(TRANSITION_FAILURE_DETAIL),
    );
    assert_eq!(failure.error().to_string(), TRANSITION_FAILURE_DETAIL);
    assert_eq!(failure.to_string(), TRANSITION_FAILURE_DETAIL);
    assert!(format!("{failure:?}").contains(TRANSITION_FAILURE_DETAIL));
    assert_eq!(
        failure.source().unwrap().to_string(),
        TRANSITION_FAILURE_DETAIL
    );
    let (state, source) = failure.into_parts();
    assert_eq!(state, FIRST_TRANSITION_STATE);
    assert_eq!(source.to_string(), TRANSITION_FAILURE_DETAIL);

    let failure = ProcessTransitionError::new(
        SECOND_TRANSITION_STATE,
        anyhow::anyhow!(TRANSITION_FAILURE_DETAIL),
    );
    assert_eq!(failure.into_state(), SECOND_TRANSITION_STATE);
}
