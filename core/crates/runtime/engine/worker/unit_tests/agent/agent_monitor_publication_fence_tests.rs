use crate::worker::tests::*;
const RUNTIME_FIELD_INCREMENT: u64 = 1;
const REPLACEMENT_BOOT_ID: &str = "replacement-agent-monitor-boot";
const REPLACEMENT_SOCKET_COMPONENT: &str = "replacement-agent-monitor.sock";
const REPLACEMENT_AUTH_TOKEN: &str = "replacement-agent-monitor-token";
const REPLACEMENT_COMMAND: &str = "/usr/bin/false";
const REPLACEMENT_ARGUMENT: &str = "replacement-agent-monitor-argument";
const REPLACEMENT_CWD_COMPONENT: &str = "replacement-agent-monitor-cwd";

#[test]
fn agent_monitor_publication_fence_binds_the_current_runtime() {
    let fixture = Fixture::new();
    let expected = lock_recover(&fixture.state.record).clone();

    let fence = agent_monitor_publication_fence::acquire(&fixture.state, &expected).unwrap();
    let session_entry = fixture
        .state
        .namespace
        .lease()
        .file(&fixture.state.paths.agent_binding(fixture.terminal_id()))
        .unwrap();
    assert!(
        fixture
            .state
            .paths
            .try_acquire_session_lifecycle_lock(fixture.terminal_id())
            .unwrap()
            .is_none()
    );
    assert!(session_entry.lock_parent_exclusive().is_err());
    drop(fence);
    assert!(
        fixture
            .state
            .paths
            .try_acquire_session_lifecycle_lock(fixture.terminal_id())
            .unwrap()
            .is_some()
    );
    assert!(session_entry.lock_parent_exclusive().is_ok());
}

#[test]
fn agent_monitor_publication_fence_rejects_a_successor_launch() {
    let fixture = Fixture::new();
    let current = lock_recover(&fixture.state.record).clone();
    let successor = WorkerLaunchSpec {
        schema_version: WORKER_LAUNCH_SCHEMA_VERSION,
        terminal_id: current.terminal_id,
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        socket_path: current.socket_path.clone(),
        auth_token: current.auth_token.clone(),
        controller_generation: current.controller_generation,
        command: current.command.clone(),
        args: current.args.clone(),
        cwd: current.cwd.clone(),
        environment: Default::default(),
        initial_input: None,
        cols: current.cols,
        rows: current.rows,
        context: None,
    };
    atomic_write_json(
        &fixture.state.paths.worker_launch(fixture.terminal_id()),
        &successor,
    )
    .unwrap();

    assert!(agent_monitor_publication_fence::acquire(&fixture.state, &current).is_err());
}

#[test]
fn agent_monitor_publication_fence_rejects_runtime_rollover() {
    let fixture = Fixture::new();
    let expected = lock_recover(&fixture.state.record).clone();
    let mut successor = expected.clone();
    successor.runtime_epoch = Uuid::new_v4();
    atomic_write_json(
        &fixture.state.paths.runtime_record(fixture.terminal_id()),
        &successor,
    )
    .unwrap();

    assert!(agent_monitor_publication_fence::acquire(&fixture.state, &expected).is_err());
}

#[test]
fn agent_monitor_publication_fence_rejects_an_archived_session() {
    let fixture = Fixture::new();
    let expected = lock_recover(&fixture.state.record).clone();
    let path = fixture.state.paths.manifest(fixture.terminal_id());
    let mut manifest: crate::model::TerminalManifest = read_json(&path).unwrap();
    manifest.archived = true;
    atomic_write_json(&path, &manifest).unwrap();

    assert!(agent_monitor_publication_fence::acquire(&fixture.state, &expected).is_err());
}

#[test]
fn agent_monitor_publication_fence_rejects_a_pending_reboot_intent() {
    let fixture = Fixture::new();
    let expected = lock_recover(&fixture.state.record).clone();
    atomic_write_json(
        &fixture
            .state
            .paths
            .reboot_migration_intent(fixture.terminal_id()),
        &serde_json::json!({}),
    )
    .unwrap();

    assert!(agent_monitor_publication_fence::acquire(&fixture.state, &expected).is_err());
}

#[test]
fn agent_monitor_publication_fence_rejects_every_runtime_authority_change() {
    let fixture = Fixture::new();
    let expected = lock_recover(&fixture.state.record).clone();
    let path = fixture.state.paths.runtime_record(fixture.terminal_id());

    for replacement in runtime_authority_replacements(&expected) {
        atomic_write_json(&path, &replacement).unwrap();
        assert!(agent_monitor_publication_fence::acquire(&fixture.state, &expected).is_err());
        atomic_write_json(&path, &expected).unwrap();
    }
}

#[test]
fn same_value_runtime_replacement_fails_before_agent_binding_commit() {
    let fixture = Fixture::new();
    let expected = lock_recover(&fixture.state.record).clone();
    let runtime = claude_hook::binding::runtime_identity(&fixture.state);
    let path = fixture.state.paths.runtime_record(fixture.terminal_id());
    let binding_path = fixture.state.paths.agent_binding(fixture.terminal_id());
    let store = AgentBindingStore::new(&binding_path);
    let fence = agent_monitor_publication_fence::acquire(&fixture.state, &expected).unwrap();

    let result = store.reconcile_detected_with_commit_fence(
        fixture.state.namespace.lease(),
        &binding(&fixture, crate::agent::AgentState::Running),
        runtime,
        || {
            atomic_write_json(&path, &expected)?;
            fence.verify()
        },
    );

    assert!(result.is_err());
    assert!(
        fixture
            .state
            .namespace
            .file_is_missing(&binding_path)
            .unwrap()
    );
}

fn runtime_authority_replacements(expected: &RuntimeRecord) -> Vec<RuntimeRecord> {
    let mut replacements = Vec::new();
    let mut push = |change: fn(&mut RuntimeRecord)| {
        let mut replacement = expected.clone();
        change(&mut replacement);
        replacements.push(replacement);
    };
    push(|record| record.worker_id = Uuid::new_v4());
    push(|record| record.worker_pid += RUNTIME_FIELD_INCREMENT as u32);
    push(|record| record.worker_start_time += RUNTIME_FIELD_INCREMENT);
    push(|record| record.child_pid += RUNTIME_FIELD_INCREMENT as u32);
    push(|record| record.child_start_time += RUNTIME_FIELD_INCREMENT);
    push(|record| record.boot_id = REPLACEMENT_BOOT_ID.into());
    push(|record| {
        record.socket_path = record
            .socket_path
            .with_file_name(REPLACEMENT_SOCKET_COMPONENT)
    });
    push(|record| record.auth_token = REPLACEMENT_AUTH_TOKEN.into());
    push(|record| record.controller_generation += RUNTIME_FIELD_INCREMENT);
    push(|record| record.started_at_ms += RUNTIME_FIELD_INCREMENT as i64);
    push(|record| record.command = REPLACEMENT_COMMAND.into());
    push(|record| record.args = vec![REPLACEMENT_ARGUMENT.into()]);
    push(|record| record.cwd = record.cwd.join(REPLACEMENT_CWD_COMPONENT));
    push(|record| record.cols += RUNTIME_FIELD_INCREMENT as u16);
    push(|record| record.rows += RUNTIME_FIELD_INCREMENT as u16);
    push(|record| record.bootstrap_delivery = BootstrapDelivery::Pending);
    replacements
}
