use super::fixture::Fixture;
use super::harness::{output, run};
use super::peer::peer;
use crate::arguments::AgentCommand as CliCommand;
use yolo_agent_adapter_contract::constants::YOLO_TERMINAL_ID_ENV;
use yolo_hub_agent_protocol::{Activity, Lifecycle};
use yolo_hub_rpc_protocol::{Command, Reply};

struct CurrentTerminal;

impl CurrentTerminal {
    fn set(terminal: &str) -> Self {
        unsafe { std::env::set_var(YOLO_TERMINAL_ID_ENV, terminal) };
        Self
    }
}

impl Drop for CurrentTerminal {
    fn drop(&mut self) {
        unsafe { std::env::remove_var(YOLO_TERMINAL_ID_ENV) };
    }
}

fn current_peer(
    fixture: &Fixture,
) -> (
    std::path::PathBuf,
    tempfile::TempDir,
    tokio::task::JoinHandle<Vec<Command>>,
) {
    let terminal = fixture.terminal("flow-1");
    let agent = fixture.agent(Activity::WaitingForInput, Lifecycle::Running {});
    peer(fixture.hub, move |command| match command {
        Command::List { .. } => Reply::Terminals {
            terminals: vec![terminal.clone()],
            next: None,
        },
        Command::ListAgents { .. } => Reply::Agents {
            agents: vec![agent.clone()],
            next: None,
        },
        Command::ListHubs { .. } => Reply::Hubs {
            hubs: vec![],
            next: None,
        },
        Command::GetAgent { .. } => Reply::Agent {
            agent: agent.clone(),
        },
        other => panic!("unexpected command {other:?}"),
    })
}

#[tokio::test]
async fn get_current_resolves_the_injected_terminal_id() {
    let _env = super::ENV_LOCK.lock().await;
    let fixture = Fixture::new();
    let (socket, _directory, server) = current_peer(&fixture);
    let _current = CurrentTerminal::set(&fixture.terminal_id.to_string());
    let outcome = run(
        socket,
        fixture.hub,
        CliCommand::Get {
            selector: None,
            current: true,
            output: output(true),
        },
    )
    .await;
    assert_eq!(outcome.code, 0);
    assert_eq!(outcome.value["terminal_name"], "flow-1");
    let commands = server.await.unwrap();
    assert!(matches!(commands.last(), Some(Command::GetAgent { .. })));
}

#[tokio::test]
async fn current_command_uses_the_injected_terminal_id() {
    let _env = super::ENV_LOCK.lock().await;
    let fixture = Fixture::new();
    let (socket, _directory, server) = current_peer(&fixture);
    let _current = CurrentTerminal::set(&fixture.terminal_id.to_string());
    let outcome = run(socket, fixture.hub, CliCommand::Current(output(true))).await;
    assert_eq!(outcome.code, 0);
    assert_eq!(outcome.value["terminal_name"], "flow-1");
    let commands = server.await.unwrap();
    assert!(matches!(commands.last(), Some(Command::GetAgent { .. })));
}
