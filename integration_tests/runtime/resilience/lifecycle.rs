use super::*;

const SESSION_EXIT_TIMEOUT: Duration = Duration::from_secs(5);
const SESSION_EXIT_POLL_INTERVAL: Duration = Duration::from_millis(25);
const STALE_TIMESTAMP_OFFSET: i64 = 1;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn complete_session_lifecycle_covers_lease_signal_terminate_recreate_and_archive() {
    let harness = Harness::start().await;
    let created = harness.create_cat("lifecycle").await;
    let id = created.manifest.terminal_id;
    let target = runtime_ref_from_view(&created);
    let initial_runtime = created.runtime.as_ref().unwrap();
    let initial_worker = ProcessIdentity {
        pid: initial_runtime.worker_pid,
        start_time: initial_runtime.worker_start_time,
    };
    assert_eq!(
        harness.client().list_sessions(false).await.unwrap().len(),
        1
    );
    assert_eq!(
        harness
            .client()
            .get_session(id)
            .await
            .unwrap()
            .manifest
            .name,
        "lifecycle"
    );
    assert_eq!(harness.client().reconcile().await.unwrap().reattached, 1);
    let lease = harness
        .client()
        .acquire_writer(target, "owner", Some(2_000))
        .await
        .unwrap();
    let renewed = harness
        .client()
        .renew_writer(&lease, target, Some(3_000))
        .await
        .unwrap();
    assert_eq!(renewed.generation, lease.generation);
    harness
        .client()
        .signal(target, lease.generation, &lease.writer_capability, "CONT")
        .await
        .unwrap();
    let snapshot = harness.client().snapshot(id).await.unwrap();
    assert_eq!(
        snapshot.runtime_epoch,
        created.runtime.as_ref().unwrap().runtime_epoch
    );
    assert!(matches!(
        harness.client().archive(id).await.unwrap_err(),
        ClientError::Remote { code, .. } if code == error_code::CONFLICT
    ));
    harness
        .client()
        .release_writer(&renewed, target)
        .await
        .unwrap();
    assert!(
        harness
            .client()
            .input(
                target,
                renewed.generation,
                &renewed.writer_capability,
                b"rejected\n".to_vec(),
            )
            .await
            .is_err()
    );
    terminate(&harness, id, "initial").await;
    let deadline = tokio::time::Instant::now() + SESSION_EXIT_TIMEOUT;
    loop {
        let view = harness.client().get_session(id).await.unwrap();
        if view
            .runtime
            .as_ref()
            .is_some_and(|runtime| runtime.status == TerminalRuntimeStatus::Exited)
        {
            break;
        }
        assert!(tokio::time::Instant::now() < deadline);
        tokio::time::sleep(SESSION_EXIT_POLL_INTERVAL).await;
    }
    wait_for_identity_gone(initial_worker).await;
    let exited = harness.client().get_session(id).await.unwrap();
    let exited_runtime = exited.runtime.as_ref().unwrap();
    let selection = yolo_runtime::model::TerminalLifecycleSelection::selected(
        id,
        exited.manifest.updated_at_ms,
        exited_runtime.status,
    )
    .with_expected_manifest_sha256(Some(
        yolo_runtime::model::TerminalManifestDigest::capture(&exited.manifest).unwrap(),
    ))
    .with_expected_runtime_epoch(Some(exited_runtime.runtime_epoch));
    let resumed = harness
        .resume_with_preconditions_and_track(selection, true)
        .await
        .unwrap();
    let resumed_runtime = resumed.runtime.as_ref().unwrap();
    let resumed_worker = ProcessIdentity {
        pid: resumed_runtime.worker_pid,
        start_time: resumed_runtime.worker_start_time,
    };
    assert_ne!(resumed_runtime.runtime_epoch, snapshot.runtime_epoch);
    assert_eq!(
        resumed.manifest.recovery_state,
        yolo_runtime::RecoveryState::Recreated
    );
    terminate(&harness, id, "resumed").await;
    wait_for_exit(&harness.client(), id).await;
    wait_for_identity_gone(resumed_worker).await;
    let archived = harness.client().archive(id).await.unwrap();
    assert!(archived.manifest.archived);
    assert!(
        harness
            .client()
            .list_sessions(false)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(harness.client().list_sessions(true).await.unwrap().len(), 1);
}

async fn terminate(harness: &Harness, id: Uuid, phase: &str) {
    harness
        .client()
        .terminate(id, true)
        .await
        .unwrap_or_else(|error| {
            panic!(
                "{phase} termination failed: {error:?}; host_stderr={}",
                harness.host_stderr_tail()
            )
        });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conditional_lifecycle_rejects_stale_or_running_selection_without_termination() {
    let harness = Harness::start().await;
    let created = harness.create_cat("conditional-lifecycle-running").await;
    let terminal_id = created.manifest.terminal_id;
    let runtime = created.runtime.as_ref().unwrap();
    let worker = ProcessIdentity {
        pid: runtime.worker_pid,
        start_time: runtime.worker_start_time,
    };
    let updated_at_ms = created.manifest.updated_at_ms;
    let client = harness.client();

    assert!(matches!(
        client
            .discard_if_current(terminal_id, true, updated_at_ms, TerminalRuntimeStatus::Running)
            .await
            .unwrap_err(),
        ClientError::Remote { code, .. } if code == error_code::CONFLICT
    ));
    assert!(matches!(
        client
            .archive_if_current(
                terminal_id,
                updated_at_ms - STALE_TIMESTAMP_OFFSET,
                TerminalRuntimeStatus::Exited,
            )
            .await
            .unwrap_err(),
        ClientError::Remote { code, .. } if code == error_code::CONFLICT
    ));
    let current = client.get_session(terminal_id).await.unwrap();
    assert_eq!(
        current.runtime.as_ref().unwrap().status,
        TerminalRuntimeStatus::Running
    );
    assert!(worker.matches());

    client.terminate(terminal_id, true).await.unwrap();
    wait_for_exit(&client, terminal_id).await;
    wait_for_identity_gone(worker).await;
    let exited = client.get_session(terminal_id).await.unwrap();
    assert_eq!(
        exited.runtime.as_ref().unwrap().status,
        TerminalRuntimeStatus::Exited
    );
    let archived = client
        .archive_if_current(
            terminal_id,
            exited.manifest.updated_at_ms,
            TerminalRuntimeStatus::Exited,
        )
        .await
        .unwrap();
    assert!(archived.manifest.archived);
}
