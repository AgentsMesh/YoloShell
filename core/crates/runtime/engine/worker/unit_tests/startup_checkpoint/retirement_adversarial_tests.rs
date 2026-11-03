use super::retirement_test_support::*;
use super::*;
use crate::model::RuntimeRecord;

#[test]
fn durable_retirement_rejects_a_changed_successor_and_cross_session_fence() {
    let fixture = CheckpointFixture::new();
    ActiveStartupCheckpoint::begin(
        &fixture.namespace,
        &fixture.paths,
        &fixture.launch,
        gone_process(),
    )
    .unwrap();
    let successor = fixture.launch.clone();
    prepare(&fixture, &successor, None).unwrap();
    let retired = fixture.read();

    let changed = successor_for(&fixture);
    assert!(prepare(&fixture, &changed, None).is_err());
    assert_eq!(fixture.read(), retired);

    let mut cross_session = retired;
    cross_session.successor.as_mut().unwrap().terminal_id = Uuid::new_v4();
    crate::paths::atomic_write_json(&fixture.path(), &cross_session).unwrap();
    let preserved = std::fs::read(fixture.path()).unwrap();
    assert!(prepare(&fixture, &successor, None).is_err());
    assert_eq!(std::fs::read(fixture.path()).unwrap(), preserved);
}

#[test]
fn cross_fence_retirement_requires_exact_predecessor_runtime_evidence() {
    let mutations: [fn(&mut RuntimeRecord); 11] = [
        |value| value.schema_version += UNSUPPORTED_SCHEMA_VERSION_OFFSET,
        |value| value.terminal_id = Uuid::new_v4(),
        |value| value.runtime_epoch = Uuid::new_v4(),
        |value| value.worker_id = Uuid::new_v4(),
        |value| value.controller_generation -= STALE_IDENTITY_OFFSET,
        |value| value.boot_id = TEST_OTHER_BOOT_ID.into(),
        |value| value.worker_pid = INVALID_PROCESS_ID,
        |value| value.worker_start_time += STALE_IDENTITY_OFFSET,
        |value| value.child_pid = INVALID_PROCESS_ID,
        |value| value.child_start_time += STALE_IDENTITY_OFFSET,
        |value| value.child_pid = value.worker_pid,
    ];
    for mutate in mutations {
        let fixture = CheckpointFixture::new();
        let mut predecessor = activated_runtime(&fixture);
        let expected = fixture.read();
        let successor = successor_for(&fixture);
        mutate(&mut predecessor);

        assert!(prepare(&fixture, &successor, Some(&predecessor)).is_err());
        assert_eq!(fixture.read(), expected);
    }
}

#[test]
fn predecessor_controller_generation_may_advance_monotonically() {
    let fixture = CheckpointFixture::new();
    let mut predecessor = activated_runtime(&fixture);
    predecessor.controller_generation += STALE_IDENTITY_OFFSET;
    let mut stale = successor_for(&fixture);
    assert!(prepare(&fixture, &stale, Some(&predecessor)).is_err());
    stale.controller_generation = predecessor.controller_generation;

    assert_eq!(
        prepare(&fixture, &stale, Some(&predecessor)).unwrap(),
        WorkerStartupCheckpointPreparation::Prepared
    );
}

#[test]
fn retirement_file_identity_cas_preserves_a_racing_replacement() {
    let fixture = CheckpointFixture::new();
    let expected = ActiveStartupCheckpoint::begin(
        &fixture.namespace,
        &fixture.paths,
        &fixture.launch,
        gone_process(),
    )
    .unwrap()
    .evidence;
    let mut replacement = expected.clone();
    replacement.controller_generation += STALE_IDENTITY_OFFSET;

    let result = super::super::retirement::prepare_with_test_hook(
        &fixture.paths,
        &fixture.launch,
        None,
        || crate::paths::atomic_write_json(&fixture.path(), &replacement),
    );

    assert!(result.is_err());
    assert_eq!(fixture.read(), replacement);
}

#[test]
fn retired_wire_contains_only_bounded_fences_and_no_launch_secrets() {
    let fixture = CheckpointFixture::new();
    ActiveStartupCheckpoint::begin(
        &fixture.namespace,
        &fixture.paths,
        &fixture.launch,
        gone_process(),
    )
    .unwrap();
    let successor = fixture.launch.clone();
    prepare(&fixture, &successor, None).unwrap();
    let bytes = std::fs::read(fixture.path()).unwrap();
    assert!(
        bytes.len()
            <= usize::try_from(crate::paths::constants::WORKER_STARTUP_CHECKPOINT_MAX_BYTES)
                .unwrap()
    );
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let object = value.as_object().unwrap();
    let mut fields = object.keys().map(String::as_str).collect::<Vec<_>>();
    fields.sort_unstable();
    let mut expected = SANITIZED_FIELDS.to_vec();
    expected.push(SUCCESSOR_FIELD);
    expected.sort_unstable();
    assert_eq!(fields, expected);
    let successor = object[SUCCESSOR_FIELD].as_object().unwrap();
    let mut fence_fields = successor.keys().map(String::as_str).collect::<Vec<_>>();
    fence_fields.sort_unstable();
    assert_eq!(fence_fields, SANITIZED_FENCE_FIELDS);
    for secret in [
        TEST_AUTH_TOKEN.as_bytes(),
        TEST_COMMAND.as_bytes(),
        TEST_ARGUMENT.as_bytes(),
        TEST_CWD.as_bytes(),
        TEST_ENVIRONMENT_KEY.as_bytes(),
        TEST_ENVIRONMENT_VALUE.as_bytes(),
        TEST_INITIAL_INPUT,
    ] {
        assert!(!bytes.windows(secret.len()).any(|window| window == secret));
    }
}

#[test]
fn retired_authority_binds_every_sanitized_launch_contract_field() {
    let fixture = CheckpointFixture::new();
    ActiveStartupCheckpoint::begin(
        &fixture.namespace,
        &fixture.paths,
        &fixture.launch,
        gone_process(),
    )
    .unwrap();
    prepare(&fixture, &fixture.launch, None).unwrap();
    assert_eq!(
        worker_startup_checkpoint_replay_disposition(&fixture.paths, &fixture.launch).unwrap(),
        WorkerStartupCheckpointReplayDisposition::Authorized
    );
    let mutations: [fn(&mut WorkerLaunchSpec); 9] = [
        |value| value.socket_path = TEST_CWD.into(),
        |value| value.auth_token = TEST_ENVIRONMENT_VALUE.into(),
        |value| value.command = TEST_CWD.into(),
        |value| value.args.clear(),
        |value| value.cwd = TEST_COMMAND.into(),
        |value| value.environment.clear(),
        |value| value.initial_input = None,
        |value| value.cols = value.rows,
        |value| value.rows = value.cols,
    ];

    for mutate in mutations {
        let mut changed = fixture.launch.clone();
        mutate(&mut changed);
        assert_eq!(
            worker_startup_checkpoint_replay_disposition(&fixture.paths, &changed).unwrap(),
            WorkerStartupCheckpointReplayDisposition::Conflicting
        );
        assert!(
            settle_worker_startup_checkpoint_for_launch(
                &fixture.paths,
                &changed,
                WorkerStartupCheckpointPolicy::SettlePresent,
            )
            .is_err()
        );
        assert!(prepare(&fixture, &changed, None).is_err());
        assert!(
            ActiveStartupCheckpoint::begin(
                &fixture.namespace,
                &fixture.paths,
                &changed,
                fixture.worker,
            )
            .is_err()
        );
    }
}
