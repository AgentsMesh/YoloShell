use super::observe_process;
use super::test_support::*;
use std::collections::BTreeMap;
use std::path::PathBuf;

const REPLACEMENT_EVIDENCE: &str = "/srv/agent/replaced.jsonl";
const REPLACEMENT_EXECUTABLE: &str = "/opt/agent/bin/claude";
const REPLACEMENT_HOME: &str = "/srv/replaced-home";

#[test]
fn changed_argv_environment_or_evidence_snapshot_is_rejected() {
    let mut argv = selected_snapshots();
    argv.argv[1] = Some(vec![crate::agent::claude::constants::EXECUTABLE.to_owned()]);
    assert_rejected(argv);

    let mut environment = selected_snapshots();
    environment.environment[1] = EnvironmentOutcome::Selected(
        [(
            crate::agent::constants::HOME_ENV.to_owned(),
            REPLACEMENT_HOME.to_owned(),
        )]
        .into(),
    );
    assert_rejected(environment);

    let mut evidence = selected_snapshots();
    evidence.evidence_paths[1] = Some(vec![PathBuf::from(REPLACEMENT_EVIDENCE)]);
    assert_rejected(evidence);

    let mut missing_evidence = selected_snapshots();
    missing_evidence.evidence_paths[1] = None;
    assert_rejected(missing_evidence);
}

#[test]
fn missing_second_argv_or_environment_snapshot_is_rejected() {
    let mut snapshots = selected_snapshots();
    snapshots.argv[1] = None;
    assert_rejected(snapshots);

    let mut environment = selected_snapshots();
    environment.environment[1] = EnvironmentOutcome::Unavailable;
    assert_rejected(environment);
}

#[test]
fn same_pid_exec_snapshot_cannot_mix_new_argv_with_old_identity() {
    let mut after_exec = identity();
    after_exec.executable = crate::agent::claude::constants::EXECUTABLE.to_owned();
    after_exec.executable_file.path = PathBuf::from(REPLACEMENT_EXECUTABLE);
    after_exec.executable_file.inode += IDENTITY_CHANGE;
    let mut snapshots = selected_snapshots();
    snapshots.argv[1] = Some(vec![crate::agent::claude::constants::EXECUTABLE.to_owned()]);
    assert!(
        observe_process(
            &api_with_snapshots([Some(identity()), Some(after_exec)], snapshots, Vec::new(),),
            row(),
            &[],
        )
        .is_none()
    );
}

fn selected_snapshots() -> ObservationSnapshots {
    stable_snapshots(EnvironmentOutcome::Selected(BTreeMap::new()))
}

fn assert_rejected(snapshots: ObservationSnapshots) {
    assert!(
        observe_process(
            &api_with_snapshots(stable_identities(), snapshots, Vec::new()),
            row(),
            &[],
        )
        .is_none()
    );
}
