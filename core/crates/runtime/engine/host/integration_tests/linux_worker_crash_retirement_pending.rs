use super::*;

#[path = "../test_support/integration/linux_retirement_pending_child.rs"]
mod cleanup;

#[tokio::test(start_paused = true)]
async fn linux_pending_crash_retirement_preserves_recovery_state_and_exact_live_child() {
    let (fixture, child) = Fixture::with_live_child();
    let mut child = cleanup::PendingChild::retain(&fixture, child);
    let lifecycle = fixture
        .host
        .acquire_resume_lifecycle_lock(fixture.runtime.terminal_id, None)
        .await
        .unwrap();
    // This child belongs to the cgroup but has neither a matching PTY nor its
    // capability. Exact containment does not authorize signaling the survivor.
    let error =
        crate::host::linux_retirement::settle_after_lifecycle_lock(&fixture.host, &fixture.runtime)
            .await
            .unwrap_err();
    assert!(
        error.is::<crate::host::linux_retirement::Pending>(),
        "{error:#}"
    );
    let mut manifest = fixture.manifest.clone();

    let error = fixture
        .host
        .prepare_linux_crash_exit(&mut manifest)
        .await
        .unwrap_err()
        .into_explicit_error();

    assert_eq!(error.code, crate::protocol::error_code::WORKER_UNREACHABLE);
    assert!(error.retryable);
    let expected = crate::model::TerminalManifestDigest::capture(&fixture.manifest).unwrap();
    assert_eq!(
        crate::model::TerminalManifestDigest::capture(&manifest).unwrap(),
        expected
    );
    let durable = fixture
        .host
        .load_durable_manifest(manifest.terminal_id)
        .unwrap();
    assert_eq!(durable.recovery_state, fixture.manifest.recovery_state);
    assert_eq!(
        crate::model::TerminalManifestDigest::capture(&durable).unwrap(),
        expected
    );
    assert_eq!(fixture.receipt(), fixture.armed);
    assert!(!fixture.host.paths.exit_record(manifest.terminal_id).exists());
    assert!(process_identity_is_verified_live(
        fixture.runtime.child_pid,
        fixture.runtime.child_start_time
    ));
    drop(lifecycle);
    child.finish().unwrap();
    tokio::time::resume();
    fixture.settle().await;
    assert_eq!(
        fixture.receipt().retired_lease(),
        fixture.armed.armed_lease()
    );
}
