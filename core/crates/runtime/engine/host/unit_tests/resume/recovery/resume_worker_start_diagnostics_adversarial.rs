use crate::host::tests::*;
include!("../../../test_support/unit/resume/resume_worker_start_diagnostics_constants.rs");

const RESUME_WORKER_PRIVATE_PATH_COMPONENT: &str = "private-resume-worker-diagnostics";

#[tokio::test(flavor = "current_thread")]
async fn failed_rearm_replay_sanitizes_private_worker_start_diagnostics() {
    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);
    let (manifest, transaction, mut runtime) = dead_survivor_rearm_fixture(&host, directory.path());
    runtime.bootstrap_delivery = BootstrapDelivery::Failed;
    host.write_namespace_json(&host.paths.runtime_record(manifest.terminal_id), &runtime)
        .unwrap();
    persist_failed_resume_checkpoint(&host, &transaction, &runtime);
    let private_path = directory.path().join(RESUME_WORKER_PRIVATE_PATH_COMPONENT);
    let raw_detail = private_path.to_string_lossy().into_owned();
    let raw_for_hook = raw_detail.clone();
    let hook = creation_checkpoint_test_hook::install(move || {
        Err(anyhow::Error::msg(raw_for_hook.clone()).context(RESUME_WORKER_PRIVATE_TOKEN))
    });

    let error = host
        .resume_session_locked(
            manifest.terminal_id,
            ResumeSelection::Preserve(ResumeIntent::Automatic),
        )
        .await
        .unwrap_err()
        .into_explicit_error();
    drop(hook);

    assert_eq!(error.code, crate::protocol::error_code::WORKER_START_FAILED);
    assert_eq!(error.message, WORKER_START_PUBLIC_DETAIL);
    assert!(!error.message.contains(&raw_detail));
    assert!(!error.message.contains(RESUME_WORKER_PRIVATE_TOKEN));
    assert!(
        !error
            .message
            .contains(private_path.to_string_lossy().as_ref())
    );
    let replacement: ResumeTransaction =
        read_json(&host.paths.resume_transaction(manifest.terminal_id)).unwrap();
    assert_ne!(
        replacement.transaction_digest,
        transaction.transaction_digest
    );
    assert_eq!(replacement.previous_runtime_epoch, runtime.runtime_epoch);
    assert_eq!(
        host.load_durable_manifest(manifest.terminal_id)
            .unwrap()
            .recovery_state,
        RecoveryState::Uncertain
    );
}
