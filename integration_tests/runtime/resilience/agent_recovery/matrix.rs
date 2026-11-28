use super::*;
#[path = "assertions.rs"]
mod assertions;
use assertions::*;

const CREATE_SESSION_FAILURE_DETAIL: &str = "agent matrix shell creation failed";
#[path = "explicit_fail_closed.rs"]
mod explicit_fail_closed;

#[path = "invocation_assertions.rs"]
mod invocation_assertions;
use invocation_assertions::*;

#[path = "process_assertions.rs"]
mod process_assertions;
use process_assertions::*;

#[path = "runtime_settlement.rs"]
mod runtime_settlement;
use runtime_settlement::*;

#[path = "resume_retry.rs"]
mod resume_retry;
use resume_retry::*;

#[derive(Clone, Copy)]
enum AutomaticExpectation {
    Provider,
    ExplicitOnly,
    FailClosed,
}

pub(super) struct RecoveryFixture {
    pub(super) previous: TerminalView,
    pub(super) state: AgentState,
    expectation: AutomaticExpectation,
}

impl RecoveryFixture {
    pub(super) fn revoked(&self) -> bool {
        matches!(self.expectation, AutomaticExpectation::FailClosed)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn codex_reboot_obeys_the_durable_agent_state_matrix() {
    run_reboot_matrix(CODEX_PROVIDER).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn claude_reboot_obeys_the_durable_agent_state_matrix() {
    run_reboot_matrix(CLAUDE_PROVIDER).await;
}

async fn run_reboot_matrix(provider: ProviderFixture) {
    let home = tempdir().unwrap();
    let paths = Paths::new(home.path());
    paths.ensure_base().unwrap();
    let data_home = home.path().join(provider.data_home_directory);
    let bin = home.path().join(AGENT_BIN_DIR);
    std::fs::create_dir_all(&bin).unwrap();
    install_fake_agent(provider, &bin.join(provider.executable));
    let host = spawn_agent_host(home.path(), provider, &bin);
    let mut harness = Harness::start_with_host(home, host).await;
    let client = harness.client();
    let cases = [
        (AgentState::Running, AutomaticExpectation::Provider),
        (AgentState::WaitingForInput, AutomaticExpectation::Provider),
        (AgentState::Exited, AutomaticExpectation::ExplicitOnly),
        (AgentState::Unknown, AutomaticExpectation::Provider),
        (AgentState::Unknown, AutomaticExpectation::FailClosed),
    ];
    let mut fixtures = Vec::with_capacity(cases.len());
    for (index, (state, expectation)) in cases.into_iter().enumerate() {
        fixtures.push(
            create_detected_agent(&harness, &client, provider, index, state, expectation).await,
        );
    }

    let records: Vec<_> = fixtures
        .iter()
        .map(|fixture| runtime_record(&paths, fixture.previous.manifest.terminal_id))
        .collect();
    simulate_power_loss(&mut harness, &records).await;
    for (fixture, record) in fixtures.iter().zip(&records) {
        assert_runtime_identities_gone(record);
        assert_agent_identity_gone(&fixture.previous);
        persist_previous_boot_runtime(&paths, record);
        persist_binding_state(&paths, fixture).await;
        super::native_fixture_assertions::assert_prepared(&paths, record, fixture);
    }
    let runtime_snapshots: Vec<_> = fixtures
        .iter()
        .map(|fixture| {
            DurableRuntimeSnapshot::capture(&paths, fixture.previous.manifest.terminal_id)
        })
        .collect();

    let restored_host = spawn_agent_host(harness.home.path(), provider, &bin);
    harness.replace_host(restored_host).await;
    for (fixture, snapshot) in fixtures.iter().zip(&runtime_snapshots) {
        assert_automatic_recovery(&client, fixture, provider, &data_home, snapshot, &harness).await;
    }
    client.reconcile().await.unwrap();
    client.reconcile().await.unwrap();
    assert_automatic_invocations(provider, &data_home, &fixtures).await;
    assert_explicit_recovery_matrix(&client, provider, &fixtures, &data_home, &harness).await;
}

async fn create_detected_agent(
    harness: &Harness,
    client: &Client,
    provider: ProviderFixture,
    index: usize,
    state: AgentState,
    expectation: AutomaticExpectation,
) -> RecoveryFixture {
    let creation_token = Uuid::new_v4();
    let created = client
        .create_session(&CreateTerminal {
            creation_token,
            name: Some(format!("agent-{index}")),
            cwd: harness.home.path().to_path_buf(),
            cols: None,
            rows: None,
            initial_input: None,
            context: None,
        })
        .await
        .unwrap_or_else(|error| {
            panic!(
                "{CREATE_SESSION_FAILURE_DETAIL}: index={index}; error={error:?}; host_stderr={}",
                harness.host_stderr_tail()
            )
        });
    harness
        .finalize_creation_and_verify_replay(
            creation_token,
            created.terminal_id,
            created.creation_capability,
        )
        .await;
    let trusted = client.get_session(created.terminal_id).await.unwrap();
    harness.track_view(&trusted);
    let agent_cwd = harness
        .home
        .path()
        .join(AGENT_WORKING_DIRECTORY)
        .join(index.to_string());
    std::fs::create_dir_all(&agent_cwd).unwrap();
    let agent_cwd = agent_cwd.canonicalize().unwrap();
    let data_home = harness.home.path().join(provider.data_home_directory);
    let target = runtime_ref_from_record(&runtime_record(&harness.paths(), created.terminal_id));
    let lease = client
        .acquire_writer(target, format!("writer-{index}"), None)
        .await
        .unwrap();
    client
        .input(
            target,
            lease.generation,
            &lease.writer_capability,
            agent_start_command(provider, &data_home, &agent_cwd),
        )
        .await
        .unwrap();
    let previous = wait_for_binding(client, created.terminal_id, harness).await;
    wait_for_invocation_count(&data_home, index + INVOCATIONS_PER_RECOVERY).await;
    assert_ne!(
        previous.manifest.cwd.canonicalize().unwrap(),
        agent_cwd.canonicalize().unwrap()
    );
    assert_real_process_detection(&previous, provider, &agent_cwd, &data_home);
    RecoveryFixture {
        previous,
        state,
        expectation,
    }
}

pub(super) fn agent_start_command(provider: ProviderFixture, data_home: &Path, agent_cwd: &Path) -> Vec<u8> {
    let data_home = shell_single_quote(data_home);
    let agent_cwd = shell_single_quote(agent_cwd);
    format!(
        "cd '{agent_cwd}' && export {}='{data_home}' && {}{COMMAND_LINE_ENDING}",
        provider.data_home_environment, provider.executable
    )
    .into_bytes()
}

fn shell_single_quote(path: &Path) -> String {
    path.display()
        .to_string()
        .replace('\'', SHELL_SINGLE_QUOTE_ESCAPE)
}

fn runtime_record(paths: &Paths, terminal_id: Uuid) -> RuntimeRecord {
    read_json(&paths.runtime_record(terminal_id)).unwrap()
}
