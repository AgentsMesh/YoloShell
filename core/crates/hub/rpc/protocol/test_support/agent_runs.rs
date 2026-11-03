use crate::fixtures::*;
use yolo_hub_agent_protocol::{
    AGENT_PROTOCOL_SCHEMA_VERSION, AgentStopFields, AgentStopOutcome, AgentStopRequest,
    AgentStopResult,
};
use yolo_hub_agent_run_protocol::{AgentRunSelector, AgentRunState};
use yolo_hub_rpc_protocol::{Command, Reply};

const STOP_REQUEST_ID: u128 = 7;

pub fn samples() -> Vec<(Command, Reply)> {
    vec![
        (
            Command::GetAgentRun {
                request: get_agent_run(),
            },
            Reply::AgentRun {
                run: agent_run_summary(),
            },
        ),
        (
            Command::ListAgentRuns {
                request: list_agent_runs(),
            },
            Reply::AgentRuns {
                list: yolo_hub_agent_run_protocol::ListAgentRunsResult {
                    runs: vec![agent_run_summary()],
                    next: None,
                },
            },
        ),
        (
            Command::UpdateAgentRun {
                change: update_agent_run(),
            },
            Reply::AgentRun {
                run: {
                    let mut run = agent_run_summary();
                    run.revision = 2;
                    run
                },
            },
        ),
        (
            Command::AgentStop {
                request: AgentStopRequest::new(AgentStopFields {
                    schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
                    request_id: uuid::Uuid::from_u128(STOP_REQUEST_ID),
                    selector: AgentRunSelector::Address(agent_run_summary().address),
                    force: false,
                })
                .unwrap(),
            },
            Reply::AgentStop {
                stop: AgentStopResult {
                    run: {
                        let mut run = agent_run_summary();
                        run.revision = 2;
                        run.state = AgentRunState::Stopped;
                        run
                    },
                    outcome: AgentStopOutcome::Terminated,
                },
            },
        ),
    ]
}
