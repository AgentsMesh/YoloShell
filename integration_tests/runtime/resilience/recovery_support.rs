use super::*;

const HOST_RESTART_RECOVERY_TIMEOUT: Duration = HOST_READY_TIMEOUT;
const HOST_RESTART_RECOVERY_TIMEOUT_DETAIL: &str =
    "restarted Host did not adopt the surviving runtime";
const WORKER_CRASH_RETIREMENT_DETAIL: &str = "Host did not settle a crashed shell runtime";
const IMMEDIATE_REJECTION_DIAGNOSTIC: &str = "immediate resume rejection";
const SELECTED_RUNTIME_MISSING: &str = "selected recovery runtime is missing";
const SELECTION_PROJECTION_DIAGNOSTIC: &str = "current recovery selection";
const RECOVERY_SELECTION_CHANGED: &str =
    "session lifecycle selection no longer matches current state";
#[path = "recovery_custody_diagnostic.rs"]
mod custody_diagnostic;

pub(super) fn exact_recovery_selection(
    view: &yolo_runtime::TerminalView,
) -> yolo_runtime::model::TerminalLifecycleSelection {
    let runtime = view.runtime.as_ref().expect(SELECTED_RUNTIME_MISSING);
    let selection = yolo_runtime::model::TerminalLifecycleSelection::selected(
        view.manifest.terminal_id,
        view.manifest.updated_at_ms,
        runtime.status,
    )
    .with_expected_manifest_sha256(Some(
        yolo_runtime::model::TerminalManifestDigest::capture(&view.manifest).unwrap(),
    ))
    .with_expected_runtime_epoch(Some(runtime.runtime_epoch));
    assert!(selection.is_exact_catalog_selection());
    selection
}

pub(super) fn report_recovery_selection(harness: &Harness, view: &yolo_runtime::TerminalView) {
    let path = harness.paths().manifest(view.manifest.terminal_id);
    let durable: yolo_runtime::TerminalManifest = read_json(&path).unwrap();
    let mut normalized = view.manifest.clone();
    normalized.recovery_state = durable.recovery_state;
    let digest = |manifest: &yolo_runtime::TerminalManifest| {
        yolo_runtime::model::TerminalManifestDigest::capture(manifest).unwrap()
    };
    let unchanged: yolo_runtime::TerminalManifest = read_json(&path).unwrap();
    eprintln!(
        "{SELECTION_PROJECTION_DIAGNOSTIC}: view_state={:?} durable_state={:?} digest_matches={} recovery_projection_only={} durable_unchanged={}",
        view.manifest.recovery_state,
        durable.recovery_state,
        digest(&view.manifest) == digest(&durable),
        digest(&normalized) == digest(&durable),
        digest(&unchanged) == digest(&durable),
    );
}

pub(super) async fn wait_for_restarted_runtime(
    harness: &Harness,
    terminal_id: Uuid,
) -> yolo_runtime::model::RuntimeView {
    let deadline = tokio::time::Instant::now() + HOST_RESTART_RECOVERY_TIMEOUT;
    loop {
        let runtime = harness
            .client()
            .get_session(terminal_id)
            .await
            .unwrap()
            .runtime
            .unwrap();
        if runtime.status == TerminalRuntimeStatus::Running {
            return runtime;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "{HOST_RESTART_RECOVERY_TIMEOUT_DETAIL}: runtime={runtime:?}; host_stderr={}",
            harness.host_stderr_tail()
        );
        tokio::time::sleep(HOST_READY_POLL_INTERVAL).await;
    }
}

pub(super) async fn wait_for_crashed_shell_retirement(
    harness: &Harness,
    before: &RuntimeRecord,
    rejection: Option<&str>,
) {
    let deadline = tokio::time::Instant::now() + PROCESS_EXIT_TIMEOUT;
    loop {
        let view = harness
            .client()
            .get_session(before.terminal_id)
            .await
            .unwrap();
        let runtime = view.runtime.as_ref().unwrap();
        assert_eq!(runtime.runtime_epoch, before.runtime_epoch);
        assert_eq!(
            (runtime.worker_pid, runtime.worker_start_time),
            (before.worker_pid, before.worker_start_time)
        );
        assert_eq!(
            (runtime.child_pid, runtime.child_start_time),
            (before.child_pid, before.child_start_time)
        );
        if view.manifest.recovery_state == yolo_runtime::RecoveryState::NeedsRecreate {
            assert_eq!(runtime.status, TerminalRuntimeStatus::Exited);
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "{WORKER_CRASH_RETIREMENT_DETAIL}: immediate_rejection={rejection:?}; view={view:?}; custody={}; host_stderr={}",
            custody_diagnostic::capture(harness, before),
            harness.host_stderr_tail()
        );
        tokio::time::sleep(PROCESS_POLL_INTERVAL).await;
    }
    for (pid, start_time) in [
        (before.worker_pid, before.worker_start_time),
        (before.child_pid, before.child_start_time),
    ] {
        wait_for_identity_gone(ProcessIdentity { pid, start_time }).await;
        assert_eq!(
            process::observe_process_identity(pid, start_time),
            process::ProcessIdentityObservation::Gone
        );
    }
    let custody: yolo_runtime::pty_custody::CustodyDurableRecord =
        read_json(&harness.paths().pty_custody(before.terminal_id)).unwrap();
    assert!(custody.is_valid());
    assert!(custody.runtime.matches_record(before));
    assert!(custody.retired_lease().is_some());
    let evidence = custody.retirement_evidence().unwrap();
    assert!(evidence.is_valid());
    assert_eq!(
        evidence.controller_generation(),
        before.controller_generation
    );
}

pub(super) fn assert_immediate_resume_rejected(error: ClientError) -> String {
    match error {
        ClientError::Remote { code, message, .. } => {
            eprintln!("{IMMEDIATE_REJECTION_DIAGNOSTIC}: {code}: {message}");
            // Reconciliation may retire the exact old runtime after selection
            // but before resume. Its stale selection must then be fenced.
            if code == error_code::CONFLICT {
                assert_eq!(message, RECOVERY_SELECTION_CHANGED);
            } else {
                assert_eq!(code, error_code::WORKER_UNREACHABLE);
            }
            format!("{code}: {message}")
        }
        other => panic!("unexpected immediate resume error: {other}"),
    }
}
