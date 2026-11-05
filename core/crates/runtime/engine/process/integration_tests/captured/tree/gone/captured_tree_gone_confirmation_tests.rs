use super::GoneConfirmation;
use super::commit::advance_gone_confirmation;
use crate::process::captured_tree::gone::GoneObservation;

const FIRST_EMPTY_COMPLETED_DETAIL: &str = "first empty observation completed gone proof";
const SECOND_EMPTY_PENDING_DETAIL: &str = "second consecutive empty observation stayed pending";
const RESET_OBSERVATION_COMPLETED_DETAIL: &str = "non-empty evidence completed gone proof";
const RESET_FAILED_DETAIL: &str = "non-empty evidence retained empty confirmation";

#[test]
fn two_consecutive_empty_observations_complete() {
    let (first, complete) =
        advance_gone_confirmation(&GoneConfirmation::Unconfirmed, GoneObservation::Empty);
    assert!(!complete, "{FIRST_EMPTY_COMPLETED_DETAIL}");

    let (_consumed, complete) = advance_gone_confirmation(&first, GoneObservation::Empty);
    assert!(complete, "{SECOND_EMPTY_PENDING_DETAIL}");
}

#[test]
fn nonempty_observation_resets_empty_confirmation() {
    assert_resets(GoneObservation::NonEmpty);
}

#[test]
fn indeterminate_observation_resets_empty_confirmation() {
    assert_resets(GoneObservation::Indeterminate);
}

#[test]
fn capability_pending_observation_resets_empty_confirmation() {
    assert_resets(GoneObservation::CapabilityScanPending);
    assert_resets(GoneObservation::OutputFencePending);
}

fn assert_resets(observation: GoneObservation) {
    let (first, complete) =
        advance_gone_confirmation(&GoneConfirmation::Unconfirmed, GoneObservation::Empty);
    assert!(!complete, "{FIRST_EMPTY_COMPLETED_DETAIL}");
    let (reset, complete) = advance_gone_confirmation(&first, observation);
    assert!(!complete, "{RESET_OBSERVATION_COMPLETED_DETAIL}");
    let (_first_again, complete) = advance_gone_confirmation(&reset, GoneObservation::Empty);
    assert!(!complete, "{RESET_FAILED_DETAIL}");
}
