use super::*;
use crate::process::pty_session_proof::macos_liveness::MacosCapabilityLivenessState;

#[test]
fn restarting_a_released_epoch_requires_fresh_authoritative_coverage() {
    let mut state = PtyCapabilityScanState {
        liveness: MacosCapabilityLivenessState::ReleasedAfterRootExit,
        ..PtyCapabilityScanState::default()
    };

    state.restart();

    assert_eq!(
        state.liveness,
        MacosCapabilityLivenessState::ReleasedNeedsScan
    );
    assert!(state.proves_root_exit());
}

#[test]
fn missing_pty_path_proves_exit_only_after_premature_writer_release() {
    let mut state = PtyCapabilityScanState::default();
    state.released.mark_path_missing_for_test();
    assert!(!state.proves_root_exit());

    state.liveness = MacosCapabilityLivenessState::PrematurelyReleased;
    assert!(state.proves_root_exit());
}

#[test]
fn released_epoch_transition_discards_writer_scheduling_progress() {
    let current = PtyCapabilityScanState {
        liveness: MacosCapabilityLivenessState::WriterPresent,
        ..PtyCapabilityScanState::default()
    };
    let advance =
        CapabilityScanAdvance::macos_terminal(MacosCapabilityLivenessState::ReleasedNeedsScan);

    assert!(!advance.merge_scheduling_completion(&current, true, false));
    assert!(advance.merge_scheduling_completion(&current, true, true));
}

#[test]
fn released_epoch_retains_its_completed_scheduling_pass() {
    let current = PtyCapabilityScanState {
        liveness: MacosCapabilityLivenessState::ReleasedNeedsScan,
        ..PtyCapabilityScanState::default()
    };
    let advance =
        CapabilityScanAdvance::macos_terminal(MacosCapabilityLivenessState::ReleasedNeedsScan);

    assert!(advance.merge_scheduling_completion(&current, true, false));
}
