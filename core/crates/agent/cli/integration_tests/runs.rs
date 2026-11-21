use super::fixture::Fixture;
use super::harness::{output, run};
use super::peer::peer;
use crate::arguments::{AgentCommand as CliCommand, RunState, RunsArgs};
use uuid::Uuid;
use yolo_hub_agent_run_protocol::{
    AgentRunAddress, AgentRunFilter, AgentRunSelector, AgentRunState, AgentRunSummary,
    ListAgentRunsResult,
};
use yolo_hub_identity::{AgentRunId, HubId};
use yolo_hub_rpc_protocol::{Command, Reply};

const LIMIT: u16 = 8;
const RUN_ID: u128 = 1;
const START_TOKEN: u128 = 2;
const REVISION: u64 = 1;

fn row(hub: HubId) -> AgentRunSummary {
    AgentRunSummary {
        address: AgentRunAddress {
            hub_id: hub,
            run_id: AgentRunId::from_uuid(Uuid::from_u128(RUN_ID)).unwrap(),
        },
        revision: REVISION,
        name: "alpha".into(),
        aliases: Vec::new(),
        labels: Default::default(),
        provider_id: "codex".into(),
        workspace: None,
        parent: None,
        terminal: None,
        state: AgentRunState::Launched,
        current: None,
        start_token: Uuid::from_u128(START_TOKEN),
        created_at_ms: 0,
        updated_at_ms: 0,
    }
}

fn args(show: Option<&str>, state: Vec<RunState>) -> RunsArgs {
    RunsArgs {
        show: show.map(Into::into),
        state,
        provider: None,
        parent: None,
        name: None,
        label: None,
        after: None,
        limit: LIMIT,
        output: output(true),
    }
}

#[tokio::test]
async fn runs_lists_one_page_with_one_filter() {
    let fixture = Fixture::new();
    let hub = fixture.hub;
    let replied = row(hub);
    let (socket, _directory, server) = peer(hub, move |command| match command {
        Command::ListAgentRuns { request } => {
            let fields = request.fields();
            assert_eq!(fields.limit, LIMIT);
            assert!(matches!(
                fields.filter,
                Some(AgentRunFilter::State { ref is }) if is == &[AgentRunState::Launched]
            ));
            Reply::AgentRuns {
                list: ListAgentRunsResult {
                    runs: vec![replied.clone()],
                    next: None,
                },
            }
        }
        other => panic!("unexpected command {other:?}"),
    });
    let outcome = run(
        socket,
        hub,
        CliCommand::Runs(args(None, vec![RunState::Launched])),
    )
    .await;
    assert_eq!(outcome.code, 0);
    assert_eq!(outcome.value["runs"][0]["name"], "alpha");
    assert!(outcome.text.contains("alpha Launched provider=codex"));
    server.await.unwrap();
}

#[tokio::test]
async fn runs_show_resolves_a_name_selector() {
    let fixture = Fixture::new();
    let hub = fixture.hub;
    let replied = row(hub);
    let (socket, _directory, server) = peer(hub, move |command| match command {
        Command::GetAgentRun { request } => {
            assert_eq!(
                request.fields().selector,
                AgentRunSelector::Name("alpha".into())
            );
            Reply::AgentRun {
                run: replied.clone(),
            }
        }
        other => panic!("unexpected command {other:?}"),
    });
    let outcome = run(socket, hub, CliCommand::Runs(args(Some("alpha"), Vec::new()))).await;
    assert_eq!(outcome.code, 0);
    assert_eq!(outcome.value["address"][yolo_hub_identity::field::RUN_ID], Uuid::from_u128(RUN_ID).to_string());
    assert!(outcome.text.contains("run="));
    server.await.unwrap();
}

#[tokio::test]
async fn runs_accepts_a_label_filter() {
    let fixture = Fixture::new();
    let hub = fixture.hub;
    let replied = row(hub);
    let (socket, _directory, server) = peer(hub, move |command| match command {
        Command::ListAgentRuns { request } => {
            assert!(matches!(
                request.fields().filter,
                Some(AgentRunFilter::Label { ref key, ref value })
                    if key == "team" && value == "alpha"
            ));
            Reply::AgentRuns {
                list: ListAgentRunsResult {
                    runs: vec![replied.clone()],
                    next: None,
                },
            }
        }
        other => panic!("unexpected command {other:?}"),
    });
    let mut args = args(None, Vec::new());
    args.label = Some("team=alpha".into());
    let outcome = run(socket, hub, CliCommand::Runs(args)).await;
    assert_eq!(outcome.code, 0);
    assert_eq!(outcome.value["runs"][0]["name"], "alpha");
    server.await.unwrap();
}
