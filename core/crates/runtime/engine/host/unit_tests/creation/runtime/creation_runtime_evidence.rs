use crate::host::tests::*;
use super::*;

#[tokio::test]
async fn pending_creation_dead_or_live_never_spawns_again() {
    for live in [true, false] {
        let (_directory, host, request, binding, planned, runtime) =
            pending_creation_fixture(live).await;
        let expected = if live {
            (
                CreationRuntimeEvidence::MatchingPending,
                crate::protocol::error_code::DELIVERY_UNKNOWN,
                CreationPhase::Launching,
            )
        } else {
            (
                CreationRuntimeEvidence::Failed(CreationFailureResolution::Resolved),
                crate::protocol::error_code::WORKER_START_FAILED,
                CreationPhase::SpawnFailed,
            )
        };
        assert_eq!(
            host.creation_runtime_evidence(&binding, &planned),
            expected.0
        );
        let error = host.create_session(request.clone()).await.unwrap_err();
        assert_eq!(error.code, expected.1);
        assert!(error.retryable);
        assert_creation_plan_phase(&host, &binding, &planned, expected.2);
        let stored: RuntimeRecord =
            read_json(&host.paths.runtime_record(binding.result.terminal_id)).unwrap();
        assert_eq!(stored.runtime_epoch, runtime.runtime_epoch);
        assert_eq!(stored.worker_id, runtime.worker_id);
    }
}

#[tokio::test]
async fn concurrent_pending_create_retries_preserve_the_original_launch() {
    let (_directory, host, request, binding, planned, _) = pending_creation_fixture(true).await;
    let first = host.create_session(request.clone());
    let second = host.create_session(request);
    let (first, second) = tokio::join!(first, second);
    for error in [first.unwrap_err(), second.unwrap_err()] {
        assert_eq!(error.code, crate::protocol::error_code::DELIVERY_UNKNOWN);
    }
    assert_launching_plan(&host, &binding, &planned);
}

#[tokio::test]
async fn only_resolved_explicit_failure_transitions_to_spawn_failed() {
    for live in [true, false] {
        let (_directory, mut host, request, binding, planned, mut runtime) =
            pending_creation_fixture(live).await;
        if !live {
            grant_terminal_creation_settlement_budget(&mut host);
        }
        runtime.bootstrap_delivery = BootstrapDelivery::Failed;
        atomic_write_json(
            &host.paths.runtime_record(binding.result.terminal_id),
            &runtime,
        )
        .unwrap();
        let error = host.create_session(request.clone()).await.unwrap_err();
        let expected_code = if live {
            crate::protocol::error_code::DELIVERY_UNKNOWN
        } else {
            crate::protocol::error_code::WORKER_START_FAILED
        };
        assert_eq!(error.code, expected_code);
        let expected_phase = if live {
            CreationPhase::Launching
        } else {
            CreationPhase::SpawnFailed
        };
        {
            let ledger = lock_recover(&host.creation_capabilities);
            let stored = ledger
                .bindings
                .iter()
                .find(|entry| entry.result == binding.result)
                .unwrap();
            assert_eq!(stored.phase, expected_phase);
            assert_eq!(stored.launch.as_ref(), Some(&planned));
        }
    }
}

#[tokio::test]
async fn matching_ready_evidence_is_the_only_ready_commit_path() {
    let (_directory, mut host, request, binding, planned, mut runtime) =
        pending_creation_fixture(true).await;
    grant_terminal_creation_settlement_budget(&mut host);
    runtime.bootstrap_delivery = BootstrapDelivery::NotRequired;
    atomic_write_json(
        &host.paths.runtime_record(binding.result.terminal_id),
        &runtime,
    )
    .unwrap();
    let authentication = spawn_creation_authentication_server(&runtime);
    assert_eq!(host.create_session(request).await.unwrap(), binding.result);
    authentication.await.unwrap();
    let ledger = lock_recover(&host.creation_capabilities);
    let stored = ledger
        .bindings
        .iter()
        .find(|entry| entry.result == binding.result)
        .unwrap();
    assert_eq!(stored.phase, CreationPhase::Ready);
    assert_eq!(stored.launch.as_ref(), Some(&planned));
}

#[tokio::test]
async fn unknown_corrupt_and_mismatched_evidence_fail_closed() {
    let (_directory, host, request, binding, planned, mut runtime) =
        pending_creation_fixture(false).await;
    runtime.bootstrap_delivery = BootstrapDelivery::Unknown;
    atomic_write_json(
        &host.paths.runtime_record(binding.result.terminal_id),
        &runtime,
    )
    .unwrap();
    let unknown = host.create_session(request.clone()).await.unwrap_err();
    assert_eq!(unknown.code, crate::protocol::error_code::DELIVERY_UNKNOWN);

    std::fs::write(
        host.paths.runtime_record(binding.result.terminal_id),
        b"not-json",
    )
    .unwrap();
    let corrupt = host.create_session(request.clone()).await.unwrap_err();
    assert_eq!(corrupt.code, crate::protocol::error_code::DELIVERY_UNKNOWN);

    runtime.bootstrap_delivery = BootstrapDelivery::Pending;
    runtime.auth_token = Uuid::new_v4().simple().to_string();
    atomic_write_json(
        &host.paths.runtime_record(binding.result.terminal_id),
        &runtime,
    )
    .unwrap();
    let mismatch = host.create_session(request).await.unwrap_err();
    assert_eq!(
        mismatch.code,
        crate::protocol::error_code::IDENTITY_MISMATCH
    );
    assert_launching_plan(&host, &binding, &planned);
}

#[tokio::test]
async fn creation_runtime_identity_fields_are_all_fenced() {
    let (_directory, host, _, binding, planned, mut runtime) = pending_creation_fixture(true).await;
    assert_runtime_mismatch(&host, &binding, &planned, &runtime, |value| {
        value.terminal_id = Uuid::new_v4()
    });
    assert_runtime_mismatch(&host, &binding, &planned, &runtime, |value| {
        value.runtime_epoch = Uuid::new_v4()
    });
    assert_runtime_mismatch(&host, &binding, &planned, &runtime, |value| {
        value.worker_id = Uuid::new_v4()
    });
    assert_runtime_mismatch(&host, &binding, &planned, &runtime, |value| {
        value.socket_path.push("wrong")
    });
    assert_runtime_mismatch(&host, &binding, &planned, &runtime, |value| {
        value.command.push_str("-wrong")
    });
    assert_runtime_mismatch(&host, &binding, &planned, &runtime, |value| {
        value.worker_start_time = 0
    });
    assert_runtime_mismatch(&host, &binding, &planned, &runtime, |value| {
        value.child_pid = 0
    });
    assert_runtime_mismatch(&host, &binding, &planned, &runtime, |value| {
        value.child_pid = value.worker_pid
    });
    runtime.bootstrap_delivery = BootstrapDelivery::Failed;
    atomic_write_json(
        &host.paths.runtime_record(binding.result.terminal_id),
        &runtime,
    )
    .unwrap();
    assert_eq!(
        host.creation_runtime_evidence(&binding, &planned),
        CreationRuntimeEvidence::Failed(CreationFailureResolution::Active)
    );
    runtime.worker_pid = MISSING_CREATION_WORKER_ID;
    runtime.worker_start_time = MISSING_PROCESS_START_TIME;
    runtime.child_pid = MISSING_CREATION_CHILD_ID;
    runtime.child_start_time = MISSING_PROCESS_START_TIME;
    atomic_write_json(
        &host.paths.runtime_record(binding.result.terminal_id),
        &runtime,
    )
    .unwrap();
    assert_eq!(
        host.creation_runtime_evidence(&binding, &planned),
        CreationRuntimeEvidence::Failed(CreationFailureResolution::Resolved)
    );
}
