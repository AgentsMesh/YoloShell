use super::*;
use yolo_runtime::model::{TerminalLifecycleSelection, TerminalManifestDigest};
use yolo_runtime::{RecoveryState, TerminalView};

pub(super) async fn confirm_and_recreate(
    harness: &Harness,
    previous: &RuntimeRecord,
) -> RuntimeRecord {
    let view = wait_for_confirmation(harness, previous.terminal_id).await;
    assert!(view.agent.is_none());
    let runtime = view.runtime.as_ref().expect("retired shell runtime");
    assert_eq!(runtime.runtime_epoch, previous.runtime_epoch);
    assert_eq!(runtime.status, TerminalRuntimeStatus::Exited);
    wait_for_identity_gone(runtime_worker_identity(previous)).await;
    wait_for_identity_gone(runtime_child_identity(previous)).await;
    let selection = TerminalLifecycleSelection::selected(
        view.manifest.terminal_id,
        view.manifest.updated_at_ms,
        runtime.status,
    )
    .with_expected_manifest_sha256(Some(
        TerminalManifestDigest::capture(&view.manifest).unwrap(),
    ))
    .with_expected_runtime_epoch(Some(runtime.runtime_epoch));

    let error = harness
        .client()
        .resume(previous.terminal_id, false)
        .await
        .unwrap_err();
    assert!(
        matches!(
            &error,
            ClientError::Remote { code, .. } if code == error_code::RECREATE_REQUIRED
        ),
        "unexpected unconfirmed recovery result: {error:?}; host_stderr={}",
        harness.host_stderr_tail()
    );
    tokio::time::sleep(NON_RECOVERY_WINDOW).await;
    let unchanged = harness
        .client()
        .get_session(previous.terminal_id)
        .await
        .unwrap();
    assert_eq!(
        unchanged.manifest.recovery_state,
        RecoveryState::NeedsRecreate
    );
    assert_eq!(
        TerminalManifestDigest::capture(&unchanged.manifest).unwrap(),
        TerminalManifestDigest::capture(&view.manifest).unwrap()
    );
    assert_eq!(
        unchanged.runtime.as_ref().unwrap().runtime_epoch,
        previous.runtime_epoch
    );
    let recreated = harness
        .resume_with_preconditions_and_track(selection, true)
        .await
        .unwrap();
    assert_eq!(recreated.manifest.recovery_state, RecoveryState::Recreated);
    assert!(recreated.agent.is_none());
    assert_eq!(
        recreated.runtime.as_ref().unwrap().status,
        TerminalRuntimeStatus::Running
    );
    read_json(&harness.paths().runtime_record(previous.terminal_id)).unwrap()
}

async fn wait_for_confirmation(harness: &Harness, terminal_id: Uuid) -> TerminalView {
    tokio::time::timeout(WATCHDOG_RECOVERY_TIMEOUT, async {
        loop {
            let view = harness.client().get_session(terminal_id).await.unwrap();
            if view.manifest.recovery_state == RecoveryState::NeedsRecreate {
                return view;
            }
            tokio::time::sleep(WATCHDOG_POLL_INTERVAL).await;
        }
    })
    .await
    .unwrap_or_else(|error| {
        panic!(
            "watchdog did not publish shell recreation: {error}; terminal_id={terminal_id}; host_stderr={}",
            harness.host_stderr_tail()
        )
    })
}
