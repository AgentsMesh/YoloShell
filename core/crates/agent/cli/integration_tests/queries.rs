use super::fixture::{Fixture, text_entry};
use super::harness::{output, run};
use super::peer::peer;
use crate::arguments::{AgentCommand as CliCommand, ListArgs};
use yolo_conversation_protocol::ConversationRole;
use yolo_hub_agent_protocol::{Activity, Lifecycle};
use yolo_hub_rpc_protocol::{Command, Reply};

const PAGE_LIMIT: u32 = 10;

#[tokio::test]
async fn list_reports_current_agents_with_terminal_names() {
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
        other => panic!("unexpected command {other:?}"),
    });
    let outcome = run(
        socket,
        hub,
        CliCommand::List(ListArgs {
            all: false,
            output: output(true),
        }),
    )
    .await;
    assert_eq!(outcome.code, 0);
    assert_eq!(outcome.value["agents"][0]["terminal_name"], "flow-1");
    assert_eq!(
        outcome.value["agents"][0]["agent"]["activity"],
        serde_json::to_value(Activity::WaitingForInput).unwrap()
    );
    let commands = server.await.unwrap();
    assert!(
        matches!(
            commands.as_slice(),
            [
                Command::List {
                    include_archived: false,
                    ..
                },
                Command::ListAgents { .. },
                Command::ListHubs { .. }
            ]
        ),
        "the default listing reads live sessions only: {commands:?}"
    );
}

/// Only an explicit `--all` listing reads archived terminals.
#[tokio::test]
async fn list_all_reads_the_archived_catalogue() {
    let fixture = Fixture::new();
    let hub = fixture.hub;
    let terminal = fixture.terminal("flow-1");
    let agent = fixture.agent(Activity::Idle, Lifecycle::Exited { exit_code: None });
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
        other => panic!("unexpected command {other:?}"),
    });
    let outcome = run(
        socket,
        hub,
        CliCommand::List(ListArgs {
            all: true,
            output: output(true),
        }),
    )
    .await;
    assert_eq!(outcome.code, 0);
    assert_eq!(outcome.value["agents"][0]["terminal_name"], "flow-1");
    let commands = server.await.unwrap();
    assert!(
        matches!(
            commands.as_slice(),
            [
                Command::List {
                    include_archived: true,
                    ..
                },
                Command::ListAgents { .. },
                Command::ListHubs { .. }
            ]
        ),
        "an --all listing reads the complete catalogue: {commands:?}"
    );
}

#[tokio::test]
async fn read_returns_the_latest_conversation_page() {
    let fixture = Fixture::new();
    let hub = fixture.hub;
    let terminal = fixture.terminal("flow-1");
    let agent = fixture.agent(Activity::WaitingForInput, Lifecycle::Running {});
    let descriptor = fixture.descriptor();
    let page = fixture.page(
        3,
        vec![
            text_entry(1, ConversationRole::User, "hello"),
            text_entry(2, ConversationRole::Assistant, "world"),
        ],
    );
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
        Command::ResolveConversation { .. } => Reply::Conversation {
            conversation: descriptor.clone(),
        },
        Command::ReadConversation { .. } => Reply::ConversationRead {
            reading: page.clone(),
        },
        other => panic!("unexpected command {other:?}"),
    });
    let outcome = run(
        socket,
        hub,
        CliCommand::Read {
            selector: Some("flow-1".into()),
            current: false,
            limit: PAGE_LIMIT,
            output: output(true),
        },
    )
    .await;
    assert_eq!(outcome.code, 0);
    assert_eq!(outcome.value["status"], "page");
    assert_eq!(
        outcome.value["page"]["entries"][1]["parts"][0]["text"],
        "world"
    );
    let commands = server.await.unwrap();
    assert!(matches!(
        commands.as_slice(),
        [
            Command::List { .. },
            Command::ListAgents { .. },
            Command::ListHubs { .. },
            Command::GetAgent { .. },
            Command::ResolveConversation { .. },
            Command::ReadConversation { .. }
        ]
    ));
}
