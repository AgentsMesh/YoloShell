use super::fixture::Fixture;
use super::harness::{output, run};
use super::peer::peer;
use crate::arguments::{AgentCommand as CliCommand, PromptArgs};
use yolo_hub_agent_protocol::{Activity, AgentAction, CommandOutcome, FailureCode, Lifecycle};
use yolo_hub_rpc_protocol::{Command, Reply};

fn fixture_agent(fixture: &Fixture) -> yolo_hub_agent_protocol::AgentSnapshot {
    fixture.agent(Activity::WaitingForInput, Lifecycle::Running {})
}

#[tokio::test]
async fn send_acquires_a_lease_and_reports_the_delivered_receipt() {
    let fixture = Fixture::new();
    let hub = fixture.hub;
    let terminal = fixture.terminal("flow-1");
    let agent = fixture_agent(&fixture);
    let (socket, _directory, server) = peer(hub, move |command| match command {
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
        Command::Lease { .. } => fixture.lease(command),
        Command::AgentControl { .. } => fixture.receipt(command, CommandOutcome::Delivered {}),
        Command::Release { .. } => Reply::Completed {},
        other => panic!("unexpected command {other:?}"),
    });
    let outcome = run(
        socket,
        hub,
        CliCommand::Send(PromptArgs {
            selector: Some("flow-1".into()),
            current: false,
            prompt: Some("hello agent".into()),
            stdin: false,
            take_control: false,
            output: output(true),
        }),
    )
    .await;
    assert_eq!(outcome.code, 0);
    assert_eq!(outcome.value["outcome"]["state"], "delivered");
    let commands = server.await.unwrap();
    let action = commands
        .iter()
        .find_map(|command| match command {
            Command::AgentControl { request, .. } => Some(request.fields().action.clone()),
            _ => None,
        })
        .expect("agent control was sent");
    assert!(matches!(
        action,
        AgentAction::SubmitPrompt { prompt } if prompt.text() == "hello agent"
    ));
    assert!(
        matches!(
            commands.first(),
            Some(Command::List {
                include_archived: false,
                ..
            })
        ),
        "control resolves against live sessions only: {commands:?}"
    );
    assert!(matches!(commands.last(), Some(Command::Release { .. })));
}

#[tokio::test]
async fn send_reports_a_failed_receipt_as_rejected() {
    let fixture = Fixture::new();
    let hub = fixture.hub;
    let terminal = fixture.terminal("flow-1");
    let agent = fixture_agent(&fixture);
    let (socket, _directory, _server) = peer(hub, move |command| match command {
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
        Command::Lease { .. } => fixture.lease(command),
        Command::AgentControl { .. } => fixture.receipt(
            command,
            CommandOutcome::Failed {
                code: FailureCode::PreconditionsChanged,
            },
        ),
        Command::Release { .. } => Reply::Completed {},
        other => panic!("unexpected command {other:?}"),
    });
    let outcome = run(
        socket,
        hub,
        CliCommand::Send(PromptArgs {
            selector: Some("flow-1".into()),
            current: false,
            prompt: Some("hello agent".into()),
            stdin: false,
            take_control: false,
            output: output(true),
        }),
    )
    .await;
    assert_eq!(outcome.code, 2);
    assert_eq!(outcome.value["outcome"]["state"], "failed");
}

#[tokio::test]
async fn a_lost_control_reply_is_unknown_and_never_retried() {
    let fixture = Fixture::new();
    let hub = fixture.hub;
    let terminal = fixture.terminal("flow-1");
    let agent = fixture_agent(&fixture);
    let (socket, _directory, _server) = peer(hub, move |command| match command {
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
        Command::Lease { .. } => fixture.lease(command),
        Command::AgentControl { .. } => panic!("drop the connection after dispatch"),
        other => panic!("unexpected command {other:?}"),
    });
    let outcome = run(
        socket,
        hub,
        CliCommand::Send(PromptArgs {
            selector: Some("flow-1".into()),
            current: false,
            prompt: Some("hello agent".into()),
            stdin: false,
            take_control: false,
            output: output(true),
        }),
    )
    .await;
    assert_eq!(outcome.code, 4);
    assert_eq!(outcome.value["error"]["code"], "unknown");
}
