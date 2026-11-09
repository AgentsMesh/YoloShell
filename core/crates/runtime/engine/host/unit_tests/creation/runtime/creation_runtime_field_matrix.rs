use crate::host::tests::*;
use super::*;

const CREATION_MISMATCH_ARGUMENT: &str = "creation-mismatch-argument";
const CREATION_MISMATCH_ENV_KEY: &str = "YOLO_CREATION_MISMATCH";
const CREATION_MISMATCH_ENV_VALUE: &str = "set";
pub(super) const CREATION_MISMATCH_PATH: &str = "creation-mismatch";
const CREATION_MISMATCH_INPUT: u8 = b'x';
const CREATION_SCHEMA_OFFSET: u16 = 1;
const CREATION_GENERATION_OFFSET: u64 = 1;

#[tokio::test]
async fn creation_launch_matching_rejects_every_bound_field_mutation() {
    let (_directory, host, _, binding, planned, _) = pending_creation_fixture(false).await;
    let launch: WorkerLaunchSpec =
        read_json(&host.paths.worker_launch(binding.result.terminal_id)).unwrap();
    assert!(launch_matches_binding(
        &launch,
        &binding,
        &planned,
        &host.paths
    ));

    let mutations: [fn(&mut WorkerLaunchSpec); 15] = [
        |value| value.schema_version += CREATION_SCHEMA_OFFSET,
        |value| value.terminal_id = Uuid::nil(),
        |value| value.runtime_epoch = Uuid::nil(),
        |value| value.worker_id = Uuid::nil(),
        |value| value.auth_token.clear(),
        |value| value.socket_path.push(CREATION_MISMATCH_PATH),
        |value| value.controller_generation = 0,
        |value| value.command.push_str(CREATION_MISMATCH_ARGUMENT),
        |value| value.args.push(CREATION_MISMATCH_ARGUMENT.into()),
        |value| value.cwd.push(CREATION_MISMATCH_PATH),
        |value| value.cols = value.cols.saturating_add(1),
        |value| value.rows = value.rows.saturating_add(1),
        |value| {
            value.environment.insert(
                CREATION_MISMATCH_ENV_KEY.into(),
                CREATION_MISMATCH_ENV_VALUE.into(),
            );
        },
        |value| value.initial_input = Some(vec![CREATION_MISMATCH_INPUT]),
        |value| value.terminal_id = Uuid::new_v4(),
    ];
    for mutate in mutations {
        let mut candidate = launch.clone();
        mutate(&mut candidate);
        assert!(!launch_matches_binding(
            &candidate,
            &binding,
            &planned,
            &host.paths
        ));
    }
}

#[tokio::test]
async fn creation_runtime_matching_rejects_every_bound_field_mutation() {
    let (_directory, host, _, binding, planned, runtime) = pending_creation_fixture(false).await;
    let launch: WorkerLaunchSpec =
        read_json(&host.paths.worker_launch(binding.result.terminal_id)).unwrap();
    assert!(runtime_matches_binding(
        &runtime,
        Some(&launch),
        &binding,
        &planned,
        &host.paths
    ));

    let mutations: [fn(&mut RuntimeRecord); 18] = [
        |value| value.schema_version += CREATION_SCHEMA_OFFSET,
        |value| value.terminal_id = Uuid::nil(),
        |value| value.runtime_epoch = Uuid::nil(),
        |value| value.worker_id = Uuid::nil(),
        |value| value.auth_token.clear(),
        |value| value.socket_path.push(CREATION_MISMATCH_PATH),
        |value| value.controller_generation = 0,
        |value| value.boot_id.clear(),
        |value| value.started_at_ms = 0,
        |value| value.command.push_str(CREATION_MISMATCH_ARGUMENT),
        |value| value.args.push(CREATION_MISMATCH_ARGUMENT.into()),
        |value| value.cwd.push(CREATION_MISMATCH_PATH),
        |value| value.cols = u16::default(),
        |value| value.rows = u16::default(),
        |value| value.worker_pid = 0,
        |value| value.worker_start_time = 0,
        |value| value.child_pid = 0,
        |value| value.child_start_time = 0,
    ];
    for mutate in mutations {
        let mut candidate = runtime.clone();
        mutate(&mut candidate);
        assert!(!runtime_matches_binding(
            &candidate,
            Some(&launch),
            &binding,
            &planned,
            &host.paths
        ));
    }

    let mut wrong_generation = launch;
    wrong_generation.controller_generation += CREATION_GENERATION_OFFSET;
    assert!(!runtime_matches_binding(
        &runtime,
        Some(&wrong_generation),
        &binding,
        &planned,
        &host.paths
    ));
}
