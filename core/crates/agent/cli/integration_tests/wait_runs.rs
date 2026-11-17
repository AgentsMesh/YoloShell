use super::fixture::Fixture;
use super::harness::{output, run};
use super::peer::peer;
use crate::arguments::{AgentCommand as CliCommand, WaitArgs};
use crate::constants::STATE_WAITING_FOR_INPUT;
use yolo_hub_agent_protocol::{Activity, AgentWaitResult, Lifecycle};
use yolo_hub_agent_run_protocol::AgentRunSelector;
use yolo_hub_rpc_protocol::{Command, Reply};

const WAIT_TIMEOUT_SECS: u64 = 5;

/// Run selectors are expanded by the Hub; the reply echoes the evaluated
/// target so the client can correlate a target it never named.
#[tokio::test]
async fn wait_by_run_accepts_the_hub_expanded_target() {
    let fixture = Fixture::new();
    let hub = fixture.hub;
    let agent = fixture.agent(Activity::WaitingForInput, Lifecycle::Running {});
    let matched = agent.clone();
    let target = agent.cursor().target();
    let (socket, _directory, server) = peer(hub, move |command| match command {
        Command::WaitAgents { request } => {
            assert!(request.targets.is_empty());
            assert_eq!(request.runs, vec![AgentRunSelector::Name("alpha".into())]);
            Reply::AgentWait {
                wait: AgentWaitResult {
                    targets: vec![target],
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
            runs: vec!["alpha".into()],
            selectors: Vec::new(),
            until: vec![STATE_WAITING_FOR_INPUT.into()],
            timeout: WAIT_TIMEOUT_SECS,
            output: output(true),
        }),
    )
    .await;
    assert_eq!(outcome.code, 0);
    assert_eq!(outcome.value["agent"]["activity"], STATE_WAITING_FOR_INPUT);
    let commands = server.await.unwrap();
    assert!(matches!(
        commands.last(),
        Some(Command::WaitAgents { .. })
    ));
}
