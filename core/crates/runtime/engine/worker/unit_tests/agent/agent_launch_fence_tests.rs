use crate::worker::tests::*;
#[test]
fn agent_launch_fence_allows_exact_retained_launch_through_both_publication_paths() {
    let fixture = Fixture::new();
    publish_agent_launch_fixture(&fixture);
    let runtime = lock_recover(&fixture.state.record).clone();
    let fence = capture_agent_launch_fence(&fixture).unwrap();
    fence
        .verify(
            fixture.state.namespace.lease(),
            &fixture.state.paths,
            &runtime,
        )
        .unwrap();
    claude_hook::native_runtime::RuntimeFence::capture(&fixture.state).unwrap();
    agent_monitor_publication_fence::acquire(&fixture.state, &runtime)
        .unwrap()
        .verify()
        .unwrap();
}

#[test]
fn agent_launch_fence_rejects_same_value_launch_replacement() {
    let fixture = Fixture::new();
    let (launch, _) = publish_agent_launch_fixture(&fixture);
    let fence = capture_agent_launch_fence(&fixture).unwrap();
    atomic_write_json(
        &fixture.state.paths.worker_launch(fixture.terminal_id()),
        &launch,
    )
    .unwrap();
    assert!(
        fence
            .verify(
                fixture.state.namespace.lease(),
                &fixture.state.paths,
                &lock_recover(&fixture.state.record),
            )
            .is_err()
    );
}

#[test]
fn agent_launch_fence_rejects_changed_launch_digest_and_runtime_identity() {
    let fixture = Fixture::new();
    let (launch, checkpoint) = publish_agent_launch_fixture(&fixture);
    let mut changed = launch.clone();
    changed.initial_input = Some(NATIVE_HOOK_SHELL_ARGUMENT.into());
    atomic_write_json(
        &fixture.state.paths.worker_launch(fixture.terminal_id()),
        &changed,
    )
    .unwrap();
    assert!(capture_agent_launch_fence(&fixture).is_err());
    atomic_write_json(
        &fixture.state.paths.worker_launch(fixture.terminal_id()),
        &launch,
    )
    .unwrap();
    let mut changed = checkpoint;
    changed.runtime_epoch = Uuid::new_v4();
    atomic_write_json(
        &fixture
            .state
            .paths
            .worker_startup_checkpoint(fixture.terminal_id()),
        &changed,
    )
    .unwrap();
    assert!(capture_agent_launch_fence(&fixture).is_err());
}

#[test]
fn agent_launch_fence_rejects_missing_or_retired_startup_evidence() {
    let fixture = Fixture::new();
    let (_, mut checkpoint) = publish_agent_launch_fixture(&fixture);
    let path = fixture
        .state
        .paths
        .worker_startup_checkpoint(fixture.terminal_id());
    std::fs::remove_file(&path).unwrap();
    assert!(capture_agent_launch_fence(&fixture).is_err());
    checkpoint.status = yolo_startup_checkpoint_contract::StartupStatus::HostSettled;
    atomic_write_json(&path, &checkpoint).unwrap();
    assert!(capture_agent_launch_fence(&fixture).is_err());
}

#[test]
fn agent_launch_fence_absence_rejects_later_launch_publication() {
    let fixture = Fixture::new();
    let fence = capture_agent_launch_fence(&fixture).unwrap();
    publish_agent_launch_fixture(&fixture);
    assert!(
        fence
            .verify(
                fixture.state.namespace.lease(),
                &fixture.state.paths,
                &lock_recover(&fixture.state.record),
            )
            .is_err()
    );
}
