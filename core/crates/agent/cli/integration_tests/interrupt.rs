use super::fixture::Fixture;
use super::harness::{output, run};
use super::peer::peer;
use crate::arguments::AgentCommand as CliCommand;
use yolo_hub_agent_protocol::{Activity, AgentAction, CommandOutcome, Lifecycle};
use yolo_hub_rpc_protocol::{Command, Reply};

fn fixture_agent(fixture: &Fixture) -> yolo_hub_agent_protocol::AgentSnapshot {
    fixture.agent(Activity::WaitingForInput, Lifecycle::Running {})
}

#[tokio::test]
async fn interrupt_sends_the_interrupt_action() {
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
        CliCommand::Interrupt {
            selector: Some("flow-1".into()),
            current: false,
            take_control: false,
            output: output(true),
        },
    )
    .await;
    assert_eq!(outcome.code, 0);
    let commands = server.await.unwrap();
    let action = commands
        .iter()
        .find_map(|command| match command {
            Command::AgentControl { request, .. } => Some(request.fields().action.clone()),
            _ => None,
        })
        .expect("agent control was sent");
    assert!(matches!(action, AgentAction::Interrupt {}));
}
