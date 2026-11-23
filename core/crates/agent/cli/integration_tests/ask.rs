use super::fixture::{Fixture, text_entry};
use super::harness::{output, run};
use super::peer::peer;
use crate::arguments::{AgentCommand as CliCommand, AskArgs};
use yolo_conversation_protocol::{ConversationReadResult, ConversationRole};
use yolo_hub_agent_protocol::{Activity, CommandOutcome, Lifecycle};
use yolo_hub_rpc_protocol::{Command, Reply};

fn ask_command(selector: &str, timeout: u64) -> CliCommand {
    CliCommand::Ask(AskArgs {
        selector: Some(selector.into()),
        current: false,
        prompt: Some("hello agent".into()),
        stdin: false,
        take_control: false,
        timeout,
        output: output(true),
    })
}

#[tokio::test]
async fn ask_waits_for_the_turn_and_returns_the_assistant_text() {
    let fixture = Fixture::new();
    let hub = fixture.hub;
    let terminal = fixture.terminal("flow-1");
    let mut get_agent_calls = 0_u8;
    let (socket, _directory, server) = peer(hub, move |command| match command {
        Command::List { .. } => Reply::Terminals {
            terminals: vec![terminal.clone()],
            next: None,
        },
        Command::ListAgents { .. } => Reply::Agents {
            agents: vec![fixture.agent(Activity::WaitingForInput, Lifecycle::Running {})],
            next: None,
        },
        Command::ListHubs { .. } => Reply::Hubs {
            hubs: vec![],
            next: None,
        },
        Command::GetAgent { .. } => {
            get_agent_calls += 1;
            let activity = if get_agent_calls == 2 {
                Activity::Working
            } else {
                Activity::WaitingForInput
            };
            Reply::Agent {
                agent: fixture.agent(activity, Lifecycle::Running {}),
            }
        }
        Command::ResolveConversation { .. } => Reply::Conversation {
            conversation: fixture.descriptor(),
        },
        Command::ReadConversation { .. } => Reply::ConversationRead {
            reading: fixture.page(2, vec![]),
        },
        Command::WatchConversation { .. } => Reply::ConversationRead {
            reading: fixture.page(
                3,
                vec![
                    text_entry(1, ConversationRole::User, "hello agent"),
                    text_entry(2, ConversationRole::Assistant, "done"),
                ],
            ),
        },
        Command::Lease { .. } => fixture.lease(command),
        Command::AgentControl { .. } => fixture.receipt(command, CommandOutcome::Delivered {}),
        Command::Release { .. } => Reply::Completed {},
        other => panic!("unexpected command {other:?}"),
    });
    let outcome = run(socket, hub, ask_command("flow-1", 30)).await;
    assert_eq!(outcome.code, 0);
    assert_eq!(outcome.value["wait"], "completed");
    assert_eq!(outcome.value["assistant_text"], "done");
    assert_eq!(outcome.value["entries"][1]["parts"][0]["text"], "done");
    let commands = server.await.unwrap();
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::WatchConversation { .. }))
    );
}

#[tokio::test]
async fn ask_times_out_without_resubmitting() {
    let fixture = Fixture::new();
    let hub = fixture.hub;
    let terminal = fixture.terminal("flow-1");
    let (socket, _directory, _server) = peer(hub, move |command| match command {
        Command::List { .. } => Reply::Terminals {
            terminals: vec![terminal.clone()],
            next: None,
        },
        Command::ListAgents { .. } => Reply::Agents {
            agents: vec![fixture.agent(Activity::WaitingForInput, Lifecycle::Running {})],
            next: None,
        },
        Command::ListHubs { .. } => Reply::Hubs {
            hubs: vec![],
            next: None,
        },
        Command::GetAgent { .. } => Reply::Agent {
            agent: fixture.agent(Activity::WaitingForInput, Lifecycle::Running {}),
        },
        Command::ResolveConversation { .. } => Reply::Conversation {
            conversation: fixture.descriptor(),
        },
        Command::ReadConversation { .. } => Reply::ConversationRead {
            reading: fixture.page(2, vec![]),
        },
        Command::WatchConversation { request, .. } => Reply::ConversationRead {
            reading: ConversationReadResult::Unchanged {
                history_generation: request.since_generation.unwrap(),
                revision: request.since_revision.unwrap(),
            },
        },
        Command::Lease { .. } => fixture.lease(command),
        Command::AgentControl { .. } => fixture.receipt(command, CommandOutcome::Delivered {}),
        Command::Release { .. } => Reply::Completed {},
        other => panic!("unexpected command {other:?}"),
    });
    let outcome = run(socket, hub, ask_command("flow-1", 1)).await;
    assert_eq!(outcome.code, 3);
    assert_eq!(outcome.value["wait"], "timeout");
}
