use super::*;

const EXPLICIT_RECREATE_FAILURE_DETAIL: &str = "explicit recreation failed";
const RECREATE_RUNTIME_MISSING_DETAIL: &str = "recreated selection runtime is missing";

pub(super) async fn assert_recreate(
    client: &Client,
    terminal_id: Uuid,
    data_home: &Path,
    invocations_before: &[fake_agent_invocation::Invocation],
    harness: &Harness,
) {
    let paths = harness.paths();
    let runtime_snapshot = DurableRuntimeSnapshot::capture(&paths, terminal_id);
    let error = client.resume(terminal_id, false).await.unwrap_err();
    assert!(matches!(
        error,
        ClientError::Remote { code, .. }
            if code == yolo_runtime::protocol::error_code::AGENT_RECOVERY_UNAVAILABLE
    ));
    runtime_snapshot.assert_unchanged(&paths);
    assert_runtime_identities_gone(&runtime_snapshot.record);
    let selected = client.get_session(terminal_id).await.unwrap();
    let selection = exact_selection(&selected);
    harness
        .resume_with_preconditions_and_track(selection, true)
        .await
        .unwrap_or_else(|error| {
            panic!(
                "{EXPLICIT_RECREATE_FAILURE_DETAIL}: {error:?}; host_stderr={}",
                harness.host_stderr_tail()
            )
        });
    let recreated =
        wait_for_running_recovery(client, terminal_id, RecoveryState::Recreated, harness).await;
    assert!(recreated.agent.is_none());
    let recreated_runtime = recreated.runtime.as_ref().expect("recreated shell runtime");
    assert_ne!(
        recreated_runtime.runtime_epoch,
        runtime_snapshot.record.runtime_epoch
    );
    assert_ne!(
        recreated_runtime.worker_id,
        runtime_snapshot.record.worker_id
    );
    assert_process_identity_rotated(
        ProcessIdentity {
            pid: runtime_snapshot.record.worker_pid,
            start_time: runtime_snapshot.record.worker_start_time,
        },
        ProcessIdentity {
            pid: recreated_runtime.worker_pid,
            start_time: recreated_runtime.worker_start_time,
        },
    );
    assert_process_identity_rotated(
        ProcessIdentity {
            pid: runtime_snapshot.record.child_pid,
            start_time: runtime_snapshot.record.child_start_time,
        },
        ProcessIdentity {
            pid: recreated_runtime.child_pid,
            start_time: recreated_runtime.child_start_time,
        },
    );
    assert_invocation_log_stable(data_home, invocations_before).await;
}

fn exact_selection(view: &TerminalView) -> yolo_runtime::model::TerminalLifecycleSelection {
    let runtime = view
        .runtime
        .as_ref()
        .expect(RECREATE_RUNTIME_MISSING_DETAIL);
    yolo_runtime::model::TerminalLifecycleSelection::selected(
        view.manifest.terminal_id,
        view.manifest.updated_at_ms,
        runtime.status,
    )
    .with_expected_manifest_sha256(Some(
        yolo_runtime::model::TerminalManifestDigest::capture(&view.manifest).unwrap(),
    ))
    .with_expected_runtime_epoch(Some(runtime.runtime_epoch))
}
