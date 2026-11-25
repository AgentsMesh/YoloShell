use crate::host::tests::*;
use super::*;

const CREATION_GATE_CONTENTION_READINESS_TIMEOUT: Duration = Duration::from_millis(25);

#[tokio::test]
async fn bound_manifest_repairs_either_matching_durable_half() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let (_, binding, _) = reserve_test_creation(&host, directory.path(), "repair");
    let manifest = host.prepare_bound_session(&binding).await.unwrap();

    std::fs::remove_file(host.paths.manifest(binding.result.terminal_id)).unwrap();
    assert!(
        host.existing_bound_manifest(&binding)
            .await
            .unwrap()
            .is_some()
    );
    assert!(host.paths.manifest(binding.result.terminal_id).exists());

    lock_recover(&host.catalog)
        .delete(binding.result.terminal_id)
        .unwrap();
    assert!(
        host.existing_bound_manifest(&binding)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        lock_recover(&host.catalog)
            .get(binding.result.terminal_id)
            .unwrap()
            .is_some()
    );

    let mut mismatched = manifest;
    mismatched.name = "wrong-session".into();
    atomic_write_json(&host.paths.manifest(binding.result.terminal_id), &mismatched).unwrap();
    assert!(host.existing_bound_manifest(&binding).await.is_err());
}

#[tokio::test]
async fn launching_replay_retries_before_startup_checkpoint_boundary() {
    for launch_file_committed in [false, true] {
        let directory = tempdir().unwrap();
        let mut config = HostConfig::new(directory.path());
        config.runtime_executable = Some(directory.path().join("missing-host"));
        let host = Host::open(config).unwrap();
        let (request, binding, ledger_path) =
            reserve_test_creation(&host, directory.path(), "launch-replay");
        host.prepare_bound_session(&binding).await.unwrap();
        let launch = lock_recover(&host.creation_capabilities)
            .prepare_launch(&ledger_path, binding.result)
            .unwrap();
        if launch_file_committed {
            atomic_write_json(
                &host.paths.worker_launch(binding.result.terminal_id),
                &planned_worker_launch(&host, &binding, &launch),
            )
            .unwrap();
        }
        assert!(
            !host
                .paths
                .worker_startup_checkpoint(binding.result.terminal_id)
                .exists()
        );

        let error = host.create_session(request).await.unwrap_err();
        assert_eq!(error.code, crate::protocol::error_code::WORKER_START_FAILED);
        let ledger = lock_recover(&host.creation_capabilities);
        let replayed = ledger
            .bindings
            .iter()
            .find(|entry| entry.result == binding.result)
            .unwrap();
        assert_eq!(replayed.phase, CreationPhase::SpawnFailed);
        assert_eq!(replayed.launch.as_ref(), Some(&launch));
    }
}

#[tokio::test]
async fn launching_replay_spawn_deadline_stays_before_checkpoint_boundary() {
    for readiness_timeout in [Duration::ZERO, CREATION_GATE_CONTENTION_READINESS_TIMEOUT] {
        let directory = tempdir().unwrap();
        let mut config = HostConfig::new(directory.path());
        config.runtime_executable = Some(directory.path().join("missing-host"));
        let host = Host::open(config).unwrap();
        let (_, binding, ledger_path) =
            reserve_test_creation(&host, directory.path(), "contended-launch-replay");
        let manifest = host.prepare_bound_session(&binding).await.unwrap();
        let launch = lock_recover(&host.creation_capabilities)
            .prepare_launch(&ledger_path, binding.result)
            .unwrap();
        let worker_launch = planned_worker_launch(&host, &binding, &launch);
        atomic_write_json(
            &host.paths.worker_launch(binding.result.terminal_id),
            &worker_launch,
        )
        .unwrap();
        let _spawn_gate = crate::process::waitable_child_spawn_guard().unwrap();

        let result = host
            .spawn_published_worker_before(
                &manifest,
                &worker_launch,
                UnstartedLaunchPolicy::Retire,
                HostOperationDeadline::after(readiness_timeout),
            )
            .await;

        assert!(result.is_err());
        assert!(!host.paths.worker_launch(binding.result.terminal_id).exists());
        assert!(
            !host
                .paths
                .worker_startup_checkpoint(binding.result.terminal_id)
                .exists()
        );
        let ledger = lock_recover(&host.creation_capabilities);
        let replayed = ledger
            .bindings
            .iter()
            .find(|entry| entry.result == binding.result)
            .unwrap()
            .clone();
        drop(ledger);
        assert_eq!(replayed.phase, CreationPhase::Launching);
        assert_eq!(replayed.launch.as_ref(), Some(&launch));
        assert!(
            host.creation_requires_launch(&ledger_path, &replayed)
                .await
                .unwrap()
        );
    }
}
