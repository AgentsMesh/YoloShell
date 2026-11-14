use super::fixture::Fixture;
use super::harness::{output, run};
use super::peer::peer;
use super::wait_agents::second_live_agent;
use crate::arguments::{AgentCommand as CliCommand, WaitArgs};
use crate::constants::{STATE_WAITING_FOR_INPUT, WAIT_MULTIPLE_HUBS};
use yolo_hub_agent_protocol::{Activity, AgentWaitResult, Lifecycle, MAX_AGENT_WAIT_MS};
use yolo_hub_rpc_protocol::{Command, Reply};

const WAIT_TIMEOUT_SECS: u64 = 5;
const REJECTED: i32 = 2;

#[tokio::test]
async fn wait_returns_the_matching_snapshot() {
    let fixture = Fixture::new();
    let hub = fixture.hub;
    let terminal = fixture.terminal("flow-1");
    let agent = fixture.agent(Activity::WaitingForInput, Lifecycle::Running {});
    let matched = agent.clone();
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
        Command::WaitAgents { request } => {
            assert_eq!(request.targets, vec![matched.cursor().target()]);
            assert!(request.condition.matches(&matched));
            assert!((1..=MAX_AGENT_WAIT_MS).contains(&request.wait_ms));
            Reply::AgentWait {
                wait: AgentWaitResult {
                    targets: request.targets.clone(),
                    matched: vec![matched.clone()],
                    ended: vec![],
                    timed_out: false,
                },
            }
        }
        other => panic!("unexpected command {other:?}"),
    });
    let outcome = run(
        socket,
        hub,
        CliCommand::Wait(WaitArgs {
            current: false,
            runs: Vec::new(),
            selectors: vec!["flow-1".into()],
            until: vec![STATE_WAITING_FOR_INPUT.into()],
            timeout: WAIT_TIMEOUT_SECS,
            output: output(true),
        }),
    )
    .await;
    assert_eq!(outcome.code, 0);
    assert_eq!(outcome.value["agent"]["activity"], STATE_WAITING_FOR_INPUT);
    let commands = server.await.unwrap();
    assert!(
        matches!(commands.last(), Some(Command::WaitAgents { .. })),
        "the wait is a single Hub long-poll: {commands:?}"
    );
}

/// Several selectors become one wait; any matching agent completes it and
/// the reply lists every match with its terminal name.
#[tokio::test]
async fn wait_matches_any_of_several_agents() {
    let fixture = Fixture::new();
    let hub = fixture.hub;
    let first_terminal = fixture.terminal("flow-1");
    let first_agent = fixture.agent(Activity::Working, Lifecycle::Running {});
    let (second_terminal, second_target) = second_live_agent(&fixture);
    let matched = second_target.clone();
    let mut terminals = vec![first_terminal, second_terminal];
    terminals.sort_by_key(|terminal| terminal.fields().address);
    let mut agents = vec![first_agent, second_target];
    agents.sort_by_key(|agent| agent.cursor().target());
    let (socket, _directory, server) = peer(hub, move |command| match command {
        Command::List { .. } => Reply::Terminals {
            terminals: terminals.clone(),
            next: None,
        },
        Command::ListAgents { .. } => Reply::Agents {
            agents: agents.clone(),
            next: None,
        },
        Command::ListHubs { .. } => Reply::Hubs {
            hubs: vec![],
            next: None,
        },
        Command::WaitAgents { request } => {
            assert_eq!(request.targets.len(), 2);
            Reply::AgentWait {
                wait: AgentWaitResult {
                    targets: request.targets.clone(),
                    matched: vec![matched.clone()],
                    ended: vec![],
                    timed_out: false,
                },
            }
        }
        other => panic!("unexpected command {other:?}"),
    });
    let outcome = run(
        socket,
        hub,
        CliCommand::Wait(WaitArgs {
            current: false,
            runs: Vec::new(),
            selectors: vec!["flow-1".into(), "flow-2".into()],
            until: vec![STATE_WAITING_FOR_INPUT.into()],
            timeout: WAIT_TIMEOUT_SECS,
            output: output(true),
        }),
    )
    .await;
    assert_eq!(outcome.code, 0);
    assert_eq!(outcome.value["agents"][0]["terminal_name"], "flow-2");
    let commands = server.await.unwrap();
    assert!(matches!(commands.last(), Some(Command::WaitAgents { .. })));
}

/// One request is addressed to one Hub, so selectors from another Hub are
/// rejected before any wait is sent.
#[tokio::test]
async fn a_wait_spanning_two_hubs_is_rejected() {
    let first = Fixture::new();
    let second = Fixture::new();
    let hub = first.hub;
    let first_terminal = first.terminal("flow-1");
    let first_agent = first.agent(Activity::WaitingForInput, Lifecycle::Running {});
    let second_terminal = second.terminal("flow-2");
    let second_agent = second.agent(Activity::WaitingForInput, Lifecycle::Running {});
    let mut terminals = vec![first_terminal, second_terminal];
    terminals.sort_by_key(|terminal| terminal.fields().address);
    let mut agents = vec![first_agent, second_agent];
    agents.sort_by_key(|agent| agent.cursor().target());
    let (socket, _directory, server) = peer(hub, move |command| match command {
        Command::List { .. } => Reply::Terminals {
            terminals: terminals.clone(),
            next: None,
        },
        Command::ListAgents { .. } => Reply::Agents {
            agents: agents.clone(),
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
        CliCommand::Wait(WaitArgs {
            current: false,
            runs: Vec::new(),
            selectors: vec!["flow-1".into(), "flow-2".into()],
            until: vec![STATE_WAITING_FOR_INPUT.into()],
            timeout: WAIT_TIMEOUT_SECS,
            output: output(true),
        }),
    )
    .await;
    assert_eq!(outcome.code, REJECTED);
    assert!(outcome.text.contains(WAIT_MULTIPLE_HUBS));
    let commands = server.await.unwrap();
    assert!(
        commands
            .iter()
            .all(|command| !matches!(command, Command::WaitAgents { .. })),
        "a mixed-Hub wait is rejected before the request: {commands:?}"
    );
}
