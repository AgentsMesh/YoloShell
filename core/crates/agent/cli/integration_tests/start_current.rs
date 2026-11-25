use super::fixture::Fixture;
use super::harness::{output, run};
use super::peer::peer;
use super::start::{started_result, terminal};
use crate::arguments::{AgentCommand as CliCommand, StartArgs};
use yolo_agent_adapter_contract::constants::{
    YOLO_HUB_ID_ENV, YOLO_RUN_ID_ENV, YOLO_TERMINAL_ID_ENV, YOLO_WORKSPACE_ID_ENV,
};
use yolo_hub_agent_run_protocol::{AgentRunAddress, AgentRunSelector};
use yolo_hub_identity::AgentRunId;
use yolo_hub_rpc_protocol::{Command, Reply};

const AWAITING_TIMEOUT: u64 = 0;

struct CallerEnvironment;

impl CallerEnvironment {
    fn set(terminal: &str, hub: &str, workspace: &str, run: Option<&str>) -> Self {
        unsafe {
            std::env::set_var(YOLO_TERMINAL_ID_ENV, terminal);
            std::env::set_var(YOLO_HUB_ID_ENV, hub);
            std::env::set_var(YOLO_WORKSPACE_ID_ENV, workspace);
            if let Some(run) = run {
                std::env::set_var(YOLO_RUN_ID_ENV, run);
            }
        }
        Self
    }
}

impl Drop for CallerEnvironment {
    fn drop(&mut self) {
        unsafe {
            std::env::remove_var(YOLO_TERMINAL_ID_ENV);
            std::env::remove_var(YOLO_HUB_ID_ENV);
            std::env::remove_var(YOLO_WORKSPACE_ID_ENV);
            std::env::remove_var(YOLO_RUN_ID_ENV);
        }
    }
}

#[tokio::test]
async fn current_start_inherits_the_injected_workspace_and_run() {
    let _env = super::ENV_LOCK.lock().await;
    let fixture = Fixture::new();
    let hub = fixture.hub;
    let terminal_id = fixture.terminal_id;
    let workspace = uuid::Uuid::from_u128(9);
    let run_id = uuid::Uuid::from_u128(11);
    let _caller = CallerEnvironment::set(
        &terminal_id.to_string(),
        &hub.to_string(),
        &workspace.to_string(),
        Some(&run_id.to_string()),
    );
    let (socket, _directory, server) = peer(hub, move |command| match command {
        Command::AgentStart { .. } => started_result(hub, terminal_id),
        Command::List { .. } => Reply::Terminals {
            terminals: vec![terminal(&fixture, false)],
            next: None,
        },
        Command::ListAgents { .. } => Reply::Agents {
            agents: vec![],
            next: None,
        },
        Command::Get { .. } => Reply::Terminal {
            terminal: terminal(&fixture, false),
        },
        Command::ListHubs { .. } => Reply::Hubs {
            hubs: vec![],
            next: None,
        },
        other => panic!("unexpected command {other:?}"),
    });
    let outcome = run(
        socket,
        hub,
        CliCommand::Start(StartArgs {
            provider: "codex".into(),
            name: None,
            cwd: Some("/tmp".into()),
            workspace: None,
            current: true,
            parent: None,
            timeout: AWAITING_TIMEOUT,
            output: output(true),
        }),
    )
    .await;
    assert_eq!(outcome.code, 0);
    let commands = server.await.unwrap();
    let inherited = commands.iter().find_map(|command| match command {
        Command::AgentStart { request } => Some(request.fields().clone()),
        _ => None,
    })
    .expect("start must launch the provider once");
    assert_eq!(inherited.workspace_id, Some(workspace));
    assert_eq!(
        inherited.parent,
        Some(AgentRunSelector::Address(AgentRunAddress {
            hub_id: hub,
            run_id: AgentRunId::from_uuid(run_id).unwrap(),
        }))
    );
}

#[tokio::test]
async fn an_explicit_parent_wins_over_the_caller_run() {
    let _env = super::ENV_LOCK.lock().await;
    let fixture = Fixture::new();
    let hub = fixture.hub;
    let terminal_id = fixture.terminal_id;
    let workspace = uuid::Uuid::from_u128(13);
    let run_id = uuid::Uuid::from_u128(17);
    let _caller = CallerEnvironment::set(
        &terminal_id.to_string(),
        &hub.to_string(),
        &workspace.to_string(),
        Some(&run_id.to_string()),
    );
    let (socket, _directory, server) = peer(hub, move |command| match command {
        Command::AgentStart { .. } => started_result(hub, terminal_id),
        Command::List { .. } => Reply::Terminals {
            terminals: vec![terminal(&fixture, false)],
            next: None,
        },
        Command::ListAgents { .. } => Reply::Agents {
            agents: vec![],
            next: None,
        },
        Command::Get { .. } => Reply::Terminal {
            terminal: terminal(&fixture, false),
        },
        Command::ListHubs { .. } => Reply::Hubs {
            hubs: vec![],
            next: None,
        },
        other => panic!("unexpected command {other:?}"),
    });
    let outcome = run(
        socket,
        hub,
        CliCommand::Start(StartArgs {
            provider: "codex".into(),
            name: None,
            cwd: Some("/tmp".into()),
            workspace: None,
            current: true,
            parent: Some("explicit-parent".into()),
            timeout: AWAITING_TIMEOUT,
            output: output(true),
        }),
    )
    .await;
    assert_eq!(outcome.code, 0);
    let commands = server.await.unwrap();
    let parent = commands
        .iter()
        .find_map(|command| match command {
            Command::AgentStart { request } => Some(request.fields().parent.clone()),
            _ => None,
        })
        .expect("start must launch the provider once");
    assert_eq!(
        parent,
        Some(AgentRunSelector::Name("explicit-parent".into()))
    );
}
