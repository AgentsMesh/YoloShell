use super::test_support::*;
use super::*;
use std::collections::BTreeMap;

#[test]
fn stable_observation_keeps_only_the_injected_selected_environment() {
    let key = crate::agent::codex::constants::DATA_HOME_ENV;
    let environment: BTreeMap<String, String> = [(key.to_owned(), TEST_HOME.to_owned())].into();
    let observed = observe_process(
        &api(
            stable_identities(),
            EnvironmentOutcome::Selected(environment.clone()),
            vec![key],
        ),
        row(),
        &[key],
    )
    .unwrap();

    assert_eq!(observed.start_time, TEST_START_TIME);
    assert_eq!(observed.environment, Some(environment));
    assert_eq!(observed.cwd, Some(PathBuf::from(TEST_CWD)));
    assert_eq!(
        observed.argv,
        [
            crate::agent::codex::constants::EXECUTABLE,
            crate::agent::codex::constants::RESUME_ARGUMENT,
        ]
    );
    assert_eq!(
        observed.evidence_paths,
        Some(vec![PathBuf::from(TEST_EVIDENCE)])
    );
}

#[test]
fn environment_unavailable_rejects_new_process_detection() {
    assert!(
        observe_process(
            &api(
                stable_identities(),
                EnvironmentOutcome::Unavailable,
                Vec::new(),
            ),
            row(),
            &[],
        )
        .is_none()
    );
}

#[test]
fn invalid_environment_rejects_the_process_instead_of_using_default_homes() {
    let observed = observe_process(
        &api(stable_identities(), EnvironmentOutcome::Invalid, Vec::new()),
        row(),
        &[],
    );
    assert!(observed.is_none());
}
