use super::fixture::Fixture;
use super::harness::{output, run};
use super::peer::peer;
use crate::arguments::{AgentCommand as CliCommand, StartArgs};
use yolo_hub_agent_protocol::{Activity, AgentStartBinding, AgentStartResult, Lifecycle};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{
    PublicError, RuntimeStatus, RuntimeSummary, TerminalSummary, TerminalSummaryFields,
};

const START_TIMEOUT: u64 = 5;
const AWAITING_TIMEOUT: u64 = 0;

#[tokio::test]
async fn start_launches_a_provider_and_waits_for_the_binding() {
    let fixture = Fixture::new();
    let hub = fixture.hub;
    let terminal_id = fixture.terminal_id;
    let mut started = false;
    let (socket, _directory, server) = peer(hub, move |command| match command {
        Command::AgentStart { .. } => {
            started = true;
            started_result(hub, terminal_id)
        }
        Command::List { .. } => Reply::Terminals {
            terminals: vec![terminal(&fixture, started)],
            next: None,
        },
        Command::ListAgents { .. } => Reply::Agents {
            agents: if started {
                vec![fixture.agent(Activity::WaitingForInput, Lifecycle::Running {})]
            } else {
                vec![]
            },
            next: None,
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
            name: Some("started".into()),
            cwd: Some("/tmp".into()),
            workspace: Some(uuid::Uuid::from_u128(7)),
            current: false,
            parent: None,
            timeout: START_TIMEOUT,
            output: output(true),
        }),
    )
    .await;
    assert_eq!(outcome.code, 0);
    assert_eq!(outcome.value["terminal_name"], "started");
    let commands = server.await.unwrap();
    let request = commands
        .iter()
        .find_map(|command| match command {
            Command::AgentStart { request } => Some(request.fields()),
            _ => None,
        })
        .expect("start must launch the provider once");
    assert_eq!(request.provider_id, "codex");
    assert_eq!(request.name.as_deref(), Some("started"));
    assert_eq!(request.cwd, "/tmp");
    assert_eq!(request.workspace_id, Some(uuid::Uuid::from_u128(7)));
}

#[tokio::test]
async fn start_reports_awaiting_binding_when_no_agent_is_observed() {
    let fixture = Fixture::new();
    let hub = fixture.hub;
    let terminal_id = fixture.terminal_id;
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
            name: Some("started".into()),
            cwd: Some("/tmp".into()),
            workspace: None,
            current: false,
            parent: None,
            timeout: AWAITING_TIMEOUT,
            output: output(true),
        }),
    )
    .await;
    assert_eq!(outcome.code, 0);
    assert_eq!(outcome.value["binding"], "awaiting");
    assert_eq!(outcome.value["provider"], "codex");
    assert_eq!(outcome.value["terminal"]["name"], "started");
    assert!(outcome.value["agent"].is_null());
    server.await.unwrap();
}

#[tokio::test]
async fn start_rejects_a_provider_the_hub_cannot_launch() {
    let fixture = Fixture::new();
    let hub = fixture.hub;
    let (socket, _directory, server) = peer(hub, move |command| match command {
        Command::AgentStart { .. } => Reply::Error {
            code: PublicError::Unsupported,
        },
        other => panic!("unexpected command {other:?}"),
    });
    let outcome = run(
        socket,
        hub,
        CliCommand::Start(StartArgs {
            provider: "not-a-provider".into(),
            name: None,
            cwd: Some("/tmp".into()),
            workspace: None,
            current: false,
            parent: None,
            timeout: START_TIMEOUT,
            output: output(true),
        }),
    )
    .await;
    assert_eq!(outcome.code, 2);
    assert_eq!(outcome.value["error"]["code"], "rejected");
    server.await.unwrap();
}

pub(super) fn started_result(
    hub: yolo_hub_identity::HubId,
    terminal_id: yolo_hub_identity::TerminalId,
) -> Reply {
    Reply::AgentStart {
        start: AgentStartResult {
            address: yolo_hub_identity::TerminalAddress {
                hub_id: hub,
                terminal_id,
            },
            provider_id: "codex".into(),
            binding: AgentStartBinding::Awaiting,
            run: None,
        },
    }
}

pub(super) fn terminal(fixture: &Fixture, started: bool) -> TerminalSummary {
    let runtime = fixture.runtime();
    TerminalSummary::new(TerminalSummaryFields {
        address: runtime.address,
        name: "started".into(),
        presentation: Default::default(),
        cwd: "/tmp".into(),
        archived: false,
        runtime: Some(RuntimeSummary {
            target: runtime,
            status: RuntimeStatus::Running,
            output_sequence: 0,
            size: None,
            safe_termination: None,
        }),
        agent: started.then(|| fixture.target()),
        recovery: None,
        workspace: None,
    })
    .unwrap()
}
