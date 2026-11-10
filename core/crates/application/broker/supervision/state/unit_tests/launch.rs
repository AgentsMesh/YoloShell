use crate::{ChildLaunch, CredentialSource, fixture_constants::*, launch_fixture};
use std::path::PathBuf;

#[test]
fn checked_launch_retains_exact_inputs_without_opening_or_creating_state() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("missing-config");
    let state = root.path().join("missing-state");
    let launch = launch_fixture::launch();
    let checked = ChildLaunch::new(
        config.clone(),
        state.clone(),
        CONFIG_REVISION.into(),
        launch.state_anchor(),
        launch.authority(),
        launch.startup_deadline().clone(),
    )
    .unwrap();
    let copy = checked.clone();
    assert_eq!(copy.config(), config);
    assert_eq!(copy.state_dir(), state);
    assert_eq!(copy.config_revision(), CONFIG_REVISION);
    assert_eq!(copy.state_anchor(), launch.state_anchor());
    assert_eq!(copy.authority(), launch.authority());
    assert_eq!(copy.startup_deadline(), launch.startup_deadline());
    assert!(!config.exists());
    assert!(!state.exists());
    assert_ne!(CredentialSource::None, CredentialSource::PrivateStdin);
}

#[test]
fn invalid_paths_and_revision_are_rejected_before_any_external_action() {
    let launch = launch_fixture::launch();
    let invalid_paths = ["relative", "/", "/tmp/../state", "/tmp/state\0suffix"];
    for invalid in invalid_paths {
        for (config, state) in [
            (PathBuf::from(invalid), PathBuf::from(STATE_PATH)),
            (PathBuf::from(CONFIG_PATH), PathBuf::from(invalid)),
        ] {
            assert!(
                ChildLaunch::new(
                    config,
                    state,
                    CONFIG_REVISION.into(),
                    launch.state_anchor(),
                    launch.authority(),
                    launch.startup_deadline().clone()
                )
                .is_err()
            );
        }
    }
    for revision in [
        String::new(),
        CONFIG_REVISION.to_uppercase(),
        "revision".into(),
    ] {
        assert!(
            ChildLaunch::new(
                CONFIG_PATH.into(),
                STATE_PATH.into(),
                revision,
                launch.state_anchor(),
                launch.authority(),
                launch.startup_deadline().clone()
            )
            .is_err()
        );
    }
}
