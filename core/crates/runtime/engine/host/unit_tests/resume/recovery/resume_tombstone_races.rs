use crate::host::tests::*;
use super::*;

#[test]
fn planned_tombstone_present_before_read_is_preserved() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let launch = shell_resume_launch(&host, &manifest);
    let previous_epoch = Uuid::new_v4();
    let transaction = shell_resume_transaction(previous_epoch, &launch);
    let replacement = crate::model::TerminationTombstone::user_request(
        manifest.terminal_id,
        launch.runtime_epoch,
        true,
    );
    let path = host.paths.terminate_tombstone(manifest.terminal_id);
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    host.write_namespace_json(&path, &replacement).unwrap();

    host.consume_resume_tombstone(&transaction).unwrap();

    assert_eq!(
        read_json::<crate::model::TerminationTombstone>(&path).unwrap(),
        replacement
    );
}

#[tokio::test]
async fn planned_tombstone_replacement_completes_and_allows_recreate() {
    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);
    let mut manifest = fixture_manifest(directory.path());
    manifest.recovery_state = RecoveryState::Restoring;
    let launch = shell_resume_launch(&host, &manifest);
    let previous_epoch = Uuid::new_v4();
    let transaction = shell_resume_transaction(previous_epoch, &launch);
    let mut child = spawn_resume_test_child();
    let runtime = planned_resume_runtime(
        &host,
        &transaction,
        std::process::id(),
        process::process_start_time(std::process::id()).unwrap(),
        child.id(),
        process::process_start_time(child.id()).unwrap(),
        BootstrapDelivery::NotRequired,
    );
    persist_host_session(&host, &manifest, &runtime);
    host.publish_resume_transaction(&transaction).unwrap();
    let marker_path = host.paths.terminate_tombstone(manifest.terminal_id);
    let previous = crate::model::TerminationTombstone::user_request(
        manifest.terminal_id,
        previous_epoch,
        false,
    );
    let replacement = crate::model::TerminationTombstone::user_request(
        manifest.terminal_id,
        runtime.runtime_epoch,
        true,
    );
    host.write_namespace_json(&marker_path, &previous).unwrap();
    let replacement_for_boundary = replacement.clone();
    let marker_for_boundary = marker_path.clone();
    *lock_recover(&host.catalog_test_probes.tombstone_consume) = Some(Box::new(move || {
        atomic_write_json(&marker_for_boundary, &replacement_for_boundary)
    }));
    let (listener, publication) =
        publish_test_socket(&host.socket_namespace, &runtime.socket_path).await;
    let server = tokio::spawn(serve_ready_resume(listener, runtime.clone()));

    let view = host
        .resume_session_locked(
            manifest.terminal_id,
            ResumeSelection::Preserve(ResumeIntent::Explicit),
        )
        .await
        .unwrap();
    server.await.unwrap();
    drop(publication);
    stop_resume_test_child(&mut child);

    assert_eq!(view.manifest.recovery_state, RecoveryState::Recreated);
    assert!(!host.paths.resume_transaction(manifest.terminal_id).exists());
    assert_eq!(
        read_json::<crate::model::TerminationTombstone>(&marker_path).unwrap(),
        replacement
    );
    let mut settled = runtime;
    settled.worker_pid = RESUME_TEST_DEAD_WORKER_PID;
    settled.worker_start_time = RESUME_TEST_PROCESS_START_TIME;
    settled.child_pid = RESUME_TEST_DEAD_CHILD_PID;
    settled.child_start_time = RESUME_TEST_PROCESS_START_TIME;
    host.write_namespace_json(&host.paths.runtime_record(manifest.terminal_id), &settled)
        .unwrap();
    persist_exit(&host, &settled);
    #[cfg(target_os = "linux")]
    crate::host::linux_retirement_evidence::publish(&host, &settled);

    let current = host
        .catalog_get(manifest.terminal_id)
        .await
        .unwrap()
        .unwrap();
    let error = host
        .resume_session(exact_recreate_request(
            &current,
            &settled,
            TerminalRuntimeStatus::Exited,
        ))
        .await
        .unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::WORKER_START_FAILED);
    let next = host
        .load_resume_transaction(manifest.terminal_id)
        .unwrap()
        .unwrap();
    assert_eq!(next.previous_runtime_epoch, settled.runtime_epoch);
}

#[tokio::test]
async fn termination_after_initial_restore_decision_blocks_transaction_publish() {
    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);
    let manifest = fixture_manifest(directory.path());
    let mut runtime = dead_resume_runtime(&host, &manifest);
    install_reaped_runtime_processes(&mut runtime);
    persist_host_session(&host, &manifest, &runtime);
    host.secondary_custody
        .install_test_retirement_evidence(&host, &runtime);
    let (reached_tx, reached_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = tokio::sync::oneshot::channel();
    *lock_recover(&host.catalog_test_probes.restore_decision) = Some(ReconcileRestoreProbe {
        reached: reached_tx,
        release: release_rx,
    });
    let reconcile = host.reconcile_manifest(manifest.clone());
    let terminate = async {
        assert_eq!(reached_rx.await.unwrap(), AutomaticRestore::Eligible);
        persist_user_termination(&host, &runtime);
        release_tx.send(()).unwrap();
    };

    let (result, ()) = tokio::join!(reconcile, terminate);

    assert_eq!(result.unreachable, 1);
    assert_eq!(result.needs_recreate, 0);
    assert!(!host.paths.resume_transaction(manifest.terminal_id).exists());
    assert!(!host.paths.worker_launch(manifest.terminal_id).exists());
    let stored = host
        .catalog_get(manifest.terminal_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.recovery_state, manifest.recovery_state);
}

fn persist_user_termination(host: &Host, runtime: &RuntimeRecord) {
    let tombstone = crate::model::TerminationTombstone::user_request(
        runtime.terminal_id,
        runtime.runtime_epoch,
        true,
    );
    host.write_namespace_json(
        &host.paths.terminate_tombstone(runtime.terminal_id),
        &tombstone,
    )
    .unwrap();
    persist_exit(host, runtime);
}

fn persist_exit(host: &Host, runtime: &RuntimeRecord) {
    let exit = ExitRecord {
        runtime_epoch: runtime.runtime_epoch,
        code: None,
        signal: None,
        reason: ExitReason::UserTerminate,
        observed_at_ms: now_ms(),
        last_output_sequence: 0,
    };
    host.write_namespace_json(&host.paths.exit_record(runtime.terminal_id), &exit)
        .unwrap();
}
