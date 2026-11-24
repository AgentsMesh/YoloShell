use crate::host::tests::*;
use super::*;

#[test]
fn resume_dead_survivor_evidence_covers_missing_live_and_retired_identities() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.recovery_state = RecoveryState::Restoring;
    let transaction = recreate_survivor_transaction(&host, &manifest, Uuid::new_v4());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    assert!(
        host.dead_resume_terminal_evidence(&manifest, &transaction)
            .unwrap()
            .is_none()
    );

    let mut child = spawn_resume_test_child();
    let live = planned_resume_runtime(
        &host,
        &transaction,
        std::process::id(),
        process::process_start_time(std::process::id()).unwrap(),
        child.id(),
        process::process_start_time(child.id()).unwrap(),
        BootstrapDelivery::Survivor,
    );
    let mut candidate = live.clone();
    candidate.runtime_epoch = Uuid::new_v4();
    host.write_namespace_json(&host.paths.runtime_record(manifest.terminal_id), &candidate)
        .unwrap();
    assert!(
        host.dead_resume_terminal_evidence(&manifest, &transaction)
            .unwrap()
            .is_none()
    );

    candidate = live.clone();
    candidate.bootstrap_delivery = BootstrapDelivery::NotRequired;
    host.write_namespace_json(&host.paths.runtime_record(manifest.terminal_id), &candidate)
        .unwrap();
    assert!(
        host.dead_resume_terminal_evidence(&manifest, &transaction)
            .unwrap()
            .is_none()
    );

    candidate = live.clone();
    candidate.auth_token.clear();
    host.write_namespace_json(&host.paths.runtime_record(manifest.terminal_id), &candidate)
        .unwrap();
    assert!(
        host.dead_resume_terminal_evidence(&manifest, &transaction)
            .is_err()
    );

    host.write_namespace_json(&host.paths.runtime_record(manifest.terminal_id), &live)
        .unwrap();
    assert!(
        host.dead_resume_terminal_evidence(&manifest, &transaction)
            .unwrap()
            .is_none()
    );

    candidate = live.clone();
    candidate.worker_pid = RESUME_TEST_DEAD_WORKER_PID;
    candidate.worker_start_time = RESUME_TEST_PROCESS_START_TIME;
    host.write_namespace_json(&host.paths.runtime_record(manifest.terminal_id), &candidate)
        .unwrap();
    assert!(
        host.dead_resume_terminal_evidence(&manifest, &transaction)
            .unwrap()
            .is_none()
    );

    candidate.child_pid = RESUME_TEST_DEAD_CHILD_PID;
    candidate.child_start_time = RESUME_TEST_PROCESS_START_TIME;
    host.write_namespace_json(&host.paths.runtime_record(manifest.terminal_id), &candidate)
        .unwrap();
    assert_eq!(
        host.dead_resume_terminal_evidence(&manifest, &transaction)
            .unwrap(),
        Some(candidate.clone())
    );

    candidate = live;
    candidate.boot_id = RESUME_TEST_PREVIOUS_BOOT_ID.into();
    host.write_namespace_json(&host.paths.runtime_record(manifest.terminal_id), &candidate)
        .unwrap();
    assert_eq!(
        host.dead_resume_terminal_evidence(&manifest, &transaction)
            .unwrap(),
        Some(candidate)
    );
    stop_resume_test_child(&mut child);
}

#[test]
fn resume_survivor_tombstone_roll_covers_intent_and_epoch_matrix() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let launch = shell_resume_launch(&host, &manifest);
    let previous_epoch = Uuid::new_v4();
    let explicit = shell_resume_transaction(previous_epoch, &launch);
    let automatic = ResumeTransaction::new(
        previous_epoch,
        ResumeIntent::Automatic,
        explicit.target.clone(),
        &launch,
    )
    .unwrap();
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    let path = host.paths.terminate_tombstone(manifest.terminal_id);
    let foreign = crate::model::TerminationTombstone::user_request(
        manifest.terminal_id,
        Uuid::new_v4(),
        false,
    );
    host.write_namespace_json(&path, &foreign).unwrap();
    host.roll_resume_tombstone(&automatic, Uuid::new_v4())
        .unwrap();
    assert_eq!(read_resume_tombstone(&host, manifest.terminal_id), foreign);

    host.namespace.remove_file(&path).unwrap();
    host.roll_resume_tombstone(&explicit, Uuid::new_v4())
        .unwrap();
    let previous = crate::model::TerminationTombstone::user_request(
        manifest.terminal_id,
        previous_epoch,
        false,
    );
    let retired_epoch = Uuid::new_v4();
    host.write_namespace_json(&path, &previous).unwrap();
    host.roll_resume_tombstone(&explicit, retired_epoch)
        .unwrap();
    assert_eq!(
        read_resume_tombstone(&host, manifest.terminal_id).runtime_epoch,
        retired_epoch
    );
    host.roll_resume_tombstone(&explicit, retired_epoch)
        .unwrap();

    host.write_namespace_json(&path, &foreign).unwrap();
    assert!(
        host.roll_resume_tombstone(&explicit, retired_epoch)
            .is_err()
    );
}
