use super::MacosProcessCapabilityObservation;
use super::state::{RootReleaseState, release_observation_from};

#[test]
fn deadline_identity_generation_change_wins_over_premature_release() {
    let observation = release_observation_from(
        RootReleaseState::IdentityGone,
        MacosProcessCapabilityObservation::PrematurelyReleased,
    );

    assert_eq!(
        observation,
        MacosProcessCapabilityObservation::ReleasedAfterRootExit
    );
}

#[test]
fn deadline_kernel_exit_state_wins_over_premature_release() {
    let observation = release_observation_from(
        RootReleaseState::Exiting,
        MacosProcessCapabilityObservation::PrematurelyReleased,
    );

    assert_eq!(
        observation,
        MacosProcessCapabilityObservation::ReleasedAfterRootExit
    );
}

#[test]
fn deadline_poison_requires_an_exact_running_root() {
    let observation = release_observation_from(
        RootReleaseState::MatchingRunning,
        MacosProcessCapabilityObservation::PrematurelyReleased,
    );

    assert_eq!(
        observation,
        MacosProcessCapabilityObservation::PrematurelyReleased
    );
}

#[test]
fn deadline_metadata_failure_is_indeterminate() {
    let observation = release_observation_from(
        RootReleaseState::Indeterminate,
        MacosProcessCapabilityObservation::PrematurelyReleased,
    );

    assert_eq!(
        observation,
        MacosProcessCapabilityObservation::Indeterminate
    );
}
