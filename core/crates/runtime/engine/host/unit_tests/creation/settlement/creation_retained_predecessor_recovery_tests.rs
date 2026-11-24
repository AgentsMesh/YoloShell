use crate::host::tests::*;
struct UnpreparedRetainedSuccessor {
    binding: CreationBinding,
    identity: CreationLaunchIdentity,
    launch: WorkerLaunchSpec,
    predecessor: RuntimeRecord,
}

fn unprepared_retained_successor(
    fixture: &CreationCheckpointFixture,
    status: crate::worker::StartupStatus,
) -> UnpreparedRetainedSuccessor {
    let checkpoint = fixture.checkpoint(
        crate::worker::StartupStage::CommandExec,
        crate::worker::StartupPhase::Entered,
        status,
        Some(checkpoint_process(CHECKPOINT_TEST_CHILD_PID)),
    );
    let predecessor = fixture.predecessor(BootstrapDelivery::Failed);
    fixture.write(&checkpoint);
    fixture.write_runtime(&predecessor);
    lock_recover(&fixture.host.creation_capabilities)
        .set_active_phase(
            &fixture.ledger_path,
            fixture.binding.result,
            CreationPhase::SpawnFailed,
        )
        .unwrap();
    let identity = lock_recover(&fixture.host.creation_capabilities)
        .prepare_launch(&fixture.ledger_path, fixture.binding.result)
        .unwrap();
    let binding = lock_recover(&fixture.host.creation_capabilities)
        .bindings
        .iter()
        .find(|value| value.result == fixture.binding.result)
        .unwrap()
        .clone();
    let launch = planned_worker_launch(&fixture.host, &binding, &identity);
    UnpreparedRetainedSuccessor {
        binding,
        identity,
        launch,
        predecessor,
    }
}

fn assert_retained_successor_prepared(host: &Host, state: &UnpreparedRetainedSuccessor) {
    let checkpoint: crate::worker::StartupCheckpointEvidence = read_json(
        &host
            .paths
            .worker_startup_checkpoint(state.binding.result.terminal_id),
    )
    .unwrap();
    assert_eq!(checkpoint.status, crate::worker::StartupStatus::HostRetired);
    assert_eq!(
        checkpoint.successor,
        Some(checkpoint_successor_fence(&state.launch))
    );
    assert!(checkpoint.predecessor_runtime_sha256.is_some());
    assert!(
        crate::worker::worker_startup_checkpoint_authorizes_retained_runtime(
            &host.paths,
            &state.launch,
            &state.predecessor,
        )
        .unwrap()
    );
}

#[tokio::test]
async fn creation_crash_after_successor_publication_prepares_active_predecessor() {
    let fixture = CreationCheckpointFixture::new(true).await;
    let state = unprepared_retained_successor(&fixture, crate::worker::StartupStatus::Active);
    fixture.host.publish_worker_launch(&state.launch).unwrap();
    let _gone = gone_checkpoint_processes();
    let config = HostConfig::new(fixture._directory.path());
    let _directory = fixture._directory;
    drop(fixture.host);
    let reopened = Host::open(config).unwrap();

    assert!(
        reopened
            .reconcile_pre_runtime_checkpoint(&state.binding, &state.identity)
            .unwrap()
    );
    assert_retained_successor_prepared(&reopened, &state);
}

#[tokio::test(start_paused = true)]
async fn creation_deadline_after_successor_publication_prepares_settled_predecessor() {
    let fixture = CreationCheckpointFixture::new(true).await;
    let state = unprepared_retained_successor(&fixture, crate::worker::StartupStatus::HostSettled);
    let deadline = HostOperationDeadline::after(CREATE_DEADLINE_TEST_TIMEOUT);
    fixture
        .host
        .publish_creation_worker_launch_before(&state.launch, deadline)
        .unwrap();
    tokio::time::advance(CREATE_DEADLINE_TEST_TIMEOUT).await;

    let Err(error) = fixture
        .host
        .spawn_published_worker_before(
            &fixture.manifest(),
            &state.launch,
            UnstartedLaunchPolicy::Retire,
            deadline,
        )
        .await
    else {
        panic!();
    };

    assert!(error.to_string().contains(WORKER_START_DEADLINE_EXPIRED));
    assert_eq!(
        fixture.read().status,
        crate::worker::StartupStatus::HostSettled
    );
    let _gone = gone_checkpoint_processes();
    assert!(
        fixture
            .host
            .reconcile_pre_runtime_checkpoint(&state.binding, &state.identity)
            .unwrap()
    );
    assert_retained_successor_prepared(&fixture.host, &state);
}

#[tokio::test]
async fn retained_predecessor_recheck_blocks_runtime_replacement_before_checkpoint_cas() {
    let fixture = CreationCheckpointFixture::new(true).await;
    let state = unprepared_retained_successor(&fixture, crate::worker::StartupStatus::Active);
    fixture.host.publish_worker_launch(&state.launch).unwrap();
    let checkpoint = fixture.read();
    let runtime_path = fixture
        .host
        .paths
        .runtime_record(state.binding.result.terminal_id);
    let mut replacement = state.predecessor.clone();
    replacement.runtime_epoch = Uuid::new_v4();
    let expected = replacement.clone();
    let hook = creation_checkpoint_test_hook::install(move || {
        atomic_write_json(&runtime_path, &replacement)
    });
    let _gone = gone_checkpoint_processes();

    let error = fixture
        .host
        .reconcile_pre_runtime_checkpoint(&state.binding, &state.identity)
        .unwrap_err();
    drop(hook);

    assert_eq!(error.code, crate::protocol::error_code::DELIVERY_UNKNOWN);
    assert_eq!(fixture.read(), checkpoint);
    assert_eq!(
        read_json::<RuntimeRecord>(
            &fixture
                .host
                .paths
                .runtime_record(state.binding.result.terminal_id)
        )
        .unwrap(),
        expected
    );
}
