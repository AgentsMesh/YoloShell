use crate::host::tests::*;
const FAILED_REARM_CRASH_DETAIL: &str = "injected failed-resume rearm crash";

async fn assert_failed_rearm_crash_reopens(
    boundary: crate::host::resume_terminal_rearm_test_hook::Boundary,
) {
    use crate::host::resume_terminal_rearm_test_hook::{Boundary, install};

    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);
    let (mut manifest, transaction, mut runtime) =
        dead_survivor_rearm_fixture(&host, directory.path());
    runtime.bootstrap_delivery = BootstrapDelivery::Failed;
    host.write_namespace_json(&host.paths.runtime_record(manifest.terminal_id), &runtime)
        .unwrap();
    let checkpoint = persist_failed_resume_checkpoint(&host, &transaction, &runtime);
    let reached = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let reached_by_hook = std::sync::Arc::clone(&reached);
    let hook = install(move |current| {
        if current == boundary {
            reached_by_hook.store(true, std::sync::atomic::Ordering::SeqCst);
            anyhow::bail!(FAILED_REARM_CRASH_DETAIL);
        }
        Ok(())
    });

    let result = host
        .rearm_dead_resume_terminal(&mut manifest, &transaction)
        .await;
    drop(hook);
    assert!(result.is_err());
    assert!(reached.load(std::sync::atomic::Ordering::SeqCst));
    assert_eq!(
        read_resume_tombstone(&host, manifest.terminal_id).runtime_epoch,
        runtime.runtime_epoch
    );
    let transaction_at_crash: ResumeTransaction =
        read_json(&host.paths.resume_transaction(manifest.terminal_id)).unwrap();
    let launch_at_crash = host
        .read_optional_resume_json::<WorkerLaunchSpec>(
            &host.paths.worker_launch(manifest.terminal_id),
        )
        .unwrap();
    assert_failed_rearm_crash_state(
        &host,
        boundary,
        &transaction,
        &transaction_at_crash,
        launch_at_crash.as_ref(),
    );
    assert_eq!(
        read_failed_rearm_checkpoint(&host, manifest.terminal_id),
        checkpoint
    );

    let terminal_id = manifest.terminal_id;
    let generation = host.controller_generation;
    let mut config = HostConfig::new(directory.path());
    config.runtime_executable = Some(directory.path().join(RESUME_TEST_MISSING_RUNTIME));
    drop(host);
    let reopened = Host::open(config).unwrap();
    assert!(reopened.controller_generation > generation);
    let result = reopened.reconcile().await;
    assert_eq!(result.unreachable, SURVIVOR_REARM_FAILED_RECONCILE_COUNT);

    let recovered: ResumeTransaction =
        read_json(&reopened.paths.resume_transaction(terminal_id)).unwrap();
    if matches!(
        boundary,
        Boundary::ReplacementTransactionPublished | Boundary::ReplacementLaunchPublished
    ) {
        assert_eq!(recovered, transaction_at_crash);
    }
    assert_ne!(recovered.transaction_digest, transaction.transaction_digest);
    assert_eq!(recovered.previous_runtime_epoch, runtime.runtime_epoch);
    assert_eq!(
        read_json::<WorkerLaunchSpec>(&reopened.paths.worker_launch(terminal_id)).unwrap(),
        recovered.launch
    );
    let retired = read_failed_rearm_checkpoint(&reopened, terminal_id);
    assert_eq!(retired.status, crate::worker::StartupStatus::HostRetired);
    assert!(retired.predecessor_runtime_sha256.is_some());
    assert_eq!(
        retired.successor,
        Some(checkpoint_successor_fence(&recovered.launch))
    );
    assert_eq!(
        reopened
            .load_durable_manifest(terminal_id)
            .unwrap()
            .recovery_state,
        RecoveryState::Uncertain
    );

    let retry = reopened
        .resume_session_locked(terminal_id, ResumeSelection::ExplicitRecreate)
        .await
        .unwrap_err()
        .into_explicit_error();
    assert_eq!(retry.code, crate::protocol::error_code::WORKER_START_FAILED);
    assert_eq!(retry.message, WORKER_START_PUBLIC_DETAIL);
    assert_eq!(
        read_json::<ResumeTransaction>(&reopened.paths.resume_transaction(terminal_id)).unwrap(),
        recovered
    );
    assert_eq!(
        read_json::<WorkerLaunchSpec>(&reopened.paths.worker_launch(terminal_id)).unwrap(),
        recovered.launch
    );
    assert_eq!(read_failed_rearm_checkpoint(&reopened, terminal_id), retired);
}

fn read_failed_rearm_checkpoint(
    host: &Host,
    terminal_id: Uuid,
) -> crate::worker::StartupCheckpointEvidence {
    read_json(&host.paths.worker_startup_checkpoint(terminal_id)).unwrap()
}

fn assert_failed_rearm_crash_state(
    host: &Host,
    boundary: crate::host::resume_terminal_rearm_test_hook::Boundary,
    previous: &ResumeTransaction,
    durable: &ResumeTransaction,
    launch: Option<&WorkerLaunchSpec>,
) {
    use crate::host::resume_terminal_rearm_test_hook::Boundary;

    let replacement_published = matches!(
        boundary,
        Boundary::ReplacementTransactionPublished | Boundary::ReplacementLaunchPublished
    );
    assert_eq!(durable == previous, !replacement_published);
    assert_eq!(
        launch,
        match boundary {
            Boundary::TombstoneRolled => Some(&previous.launch),
            Boundary::ReplacementLaunchPublished => Some(&durable.launch),
            _ => None,
        }
    );
    let expected_state = match boundary {
        Boundary::TombstoneRolled | Boundary::PreviousLaunchRemoved => RecoveryState::Restoring,
        _ => RecoveryState::Uncertain,
    };
    assert_eq!(
        host.load_durable_manifest(previous.terminal_id)
            .unwrap()
            .recovery_state,
        expected_state
    );
}

#[tokio::test]
async fn failed_rearm_reopens_after_tombstone_roll() {
    assert_failed_rearm_crash_reopens(
        crate::host::resume_terminal_rearm_test_hook::Boundary::TombstoneRolled,
    )
    .await;
}

#[tokio::test]
async fn failed_rearm_reopens_after_previous_launch_removal() {
    assert_failed_rearm_crash_reopens(
        crate::host::resume_terminal_rearm_test_hook::Boundary::PreviousLaunchRemoved,
    )
    .await;
}

#[tokio::test]
async fn failed_rearm_reopens_after_uncertain_manifest_persistence() {
    assert_failed_rearm_crash_reopens(
        crate::host::resume_terminal_rearm_test_hook::Boundary::UncertainManifestPersisted,
    )
    .await;
}

#[tokio::test]
async fn failed_rearm_reopens_after_replacement_transaction_publication() {
    assert_failed_rearm_crash_reopens(
        crate::host::resume_terminal_rearm_test_hook::Boundary::ReplacementTransactionPublished,
    )
    .await;
}

#[tokio::test]
async fn failed_rearm_reopens_after_replacement_launch_publication() {
    assert_failed_rearm_crash_reopens(
        crate::host::resume_terminal_rearm_test_hook::Boundary::ReplacementLaunchPublished,
    )
    .await;
}
