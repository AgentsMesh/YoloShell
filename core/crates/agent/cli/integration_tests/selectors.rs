use super::fixture::Fixture;
use super::harness::{output, run};
use super::peer::peer;
use crate::arguments::AgentCommand as CliCommand;
use yolo_hub_agent_protocol::{Activity, Lifecycle};
use yolo_hub_rpc_protocol::{Command, Reply};

#[tokio::test]
async fn get_resolves_a_terminal_name_to_its_current_agent() {
    let fixture = Fixture::new();
    let hub = fixture.hub;
    let terminal = fixture.terminal("flow-1");
    let agent = fixture.agent(Activity::WaitingForInput, Lifecycle::Running {});
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
        other => panic!("unexpected command {other:?}"),
    });
    let outcome = run(
        socket,
        hub,
        CliCommand::Get {
            selector: Some("flow-1".into()),
            current: false,
            output: output(true),
        },
    )
    .await;
    assert_eq!(outcome.code, 0);
    assert_eq!(outcome.value["terminal_name"], "flow-1");
    assert_eq!(
        outcome.value["agent"]["activity"],
        serde_json::to_value(Activity::WaitingForInput).unwrap()
    );
    let commands = server.await.unwrap();
    assert!(matches!(commands.last(), Some(Command::GetAgent { .. })));
}

#[tokio::test]
async fn get_resolves_a_terminal_id_to_its_current_agent() {
    let fixture = Fixture::new();
    let hub = fixture.hub;
    let selector = fixture.terminal_id.to_string();
    let terminal = fixture.terminal("flow-1");
    let agent = fixture.agent(Activity::WaitingForInput, Lifecycle::Running {});
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
        other => panic!("unexpected command {other:?}"),
    });
    let outcome = run(
        socket,
        hub,
        CliCommand::Get {
            selector: Some(selector),
            current: false,
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
async fn an_unknown_selector_is_rejected() {
    let fixture = Fixture::new();
    let hub = fixture.hub;
    let (socket, _directory, _server) = peer(hub, move |command| match command {
        Command::List { .. } => Reply::Terminals {
            terminals: vec![],
            next: None,
        },
        Command::ListAgents { .. } => Reply::Agents {
            agents: vec![],
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
        CliCommand::Get {
            selector: Some("missing".into()),
            current: false,
            output: output(true),
        },
    )
    .await;
    assert_eq!(outcome.code, 2);
    assert_eq!(outcome.value["error"]["code"], "rejected");
}
