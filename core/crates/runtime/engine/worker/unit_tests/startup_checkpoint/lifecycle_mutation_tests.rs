use super::retirement_test_support::activated_runtime;
use super::*;

#[test]
fn lifecycle_mutation_accepts_only_the_activated_runtime_or_settled_custody() {
    let fixture = CheckpointFixture::new();
    assert!(
        worker_startup_checkpoint_allows_lifecycle_mutation(
            &fixture.paths,
            fixture.launch.terminal_id,
            None,
        )
        .unwrap()
    );
    let runtime = activated_runtime(&fixture);
    assert!(
        worker_startup_checkpoint_allows_lifecycle_mutation(
            &fixture.paths,
            fixture.launch.terminal_id,
            Some(&runtime),
        )
        .unwrap()
    );
    assert!(
        !worker_startup_checkpoint_allows_lifecycle_mutation(
            &fixture.paths,
            fixture.launch.terminal_id,
            None,
        )
        .unwrap()
    );
    let mut replacement = runtime.clone();
    replacement.runtime_epoch = Uuid::new_v4();
    assert!(
        !worker_startup_checkpoint_allows_lifecycle_mutation(
            &fixture.paths,
            fixture.launch.terminal_id,
            Some(&replacement),
        )
        .unwrap()
    );

    settle_worker_startup_checkpoint_for_launch(
        &fixture.paths,
        &fixture.launch,
        WorkerStartupCheckpointPolicy::SettlePresent,
    )
    .unwrap();
    assert!(
        worker_startup_checkpoint_allows_lifecycle_mutation(
            &fixture.paths,
            fixture.launch.terminal_id,
            None,
        )
        .unwrap()
    );
}

#[test]
fn active_and_successor_authorized_checkpoints_block_lifecycle_mutation() {
    let active = CheckpointFixture::new();
    active.begin();
    assert!(
        !worker_startup_checkpoint_allows_lifecycle_mutation(
            &active.paths,
            active.launch.terminal_id,
            None,
        )
        .unwrap()
    );

    let retired = CheckpointFixture::new();
    ActiveStartupCheckpoint::begin(
        &retired.namespace,
        &retired.paths,
        &retired.launch,
        gone_process(),
    )
    .unwrap();
    settle_worker_startup_checkpoint_for_launch(
        &retired.paths,
        &retired.launch,
        WorkerStartupCheckpointPolicy::SettlePresent,
    )
    .unwrap();
    prepare_worker_startup_checkpoint_for_launch(&retired.paths, &retired.launch, None).unwrap();
    assert!(
        !worker_startup_checkpoint_allows_lifecycle_mutation(
            &retired.paths,
            retired.launch.terminal_id,
            None,
        )
        .unwrap()
    );
}
