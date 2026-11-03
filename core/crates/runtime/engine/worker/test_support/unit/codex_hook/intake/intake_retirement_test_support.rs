use super::intake_task_test_support::{IntakeRoute, IntakeTaskFixture};
use crate::codex_hooks::CodexRetirementState;
use crate::process::ProcessIdentityObservation;
use std::path::{Path, PathBuf};

pub(super) const NO_ROUTES: usize = 0;
pub(super) const SINGLE_ROUTE: usize = 1;
pub(super) const PRIVATE_FILE_MODE: u32 = crate::paths::constants::PRIVATE_FILE_MODE;

pub(super) fn observe_next(
    fixture: &IntakeTaskFixture,
    route: &IntakeRoute,
    observation: ProcessIdentityObservation,
) {
    fixture
        .router()
        .set_next_process_observation_for_test(&route.entry, observation);
}

pub(super) fn retirement_state(route: &IntakeRoute) -> CodexRetirementState {
    let admission = route.entry.admission().authority();
    admission
        .acquire_retirement_writer_fence()
        .unwrap()
        .read_state(admission)
        .unwrap()
}

pub(super) fn assert_unsealed(route: &IntakeRoute) {
    assert!(matches!(
        retirement_state(route),
        CodexRetirementState::Unsealed
    ));
}

pub(super) fn assert_sealed(route: &IntakeRoute) {
    assert!(matches!(
        retirement_state(route),
        CodexRetirementState::Sealed(_)
    ));
}

pub(super) fn assert_retired(route: &IntakeRoute) {
    assert!(matches!(
        retirement_state(route),
        CodexRetirementState::Retired(_)
    ));
}

pub(super) fn only_occurrence(directory: &Path) -> (PathBuf, Vec<u8>) {
    let mut entries = std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), SINGLE_ROUTE);
    let path = entries.pop().unwrap();
    let contents = std::fs::read(&path).unwrap();
    (path, contents)
}
