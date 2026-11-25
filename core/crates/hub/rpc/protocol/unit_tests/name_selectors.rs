use crate::commands::request;
use crate::fixtures::{agent_run_summary, workspace_summary};
use yolo_hub_agent_protocol::{
    AGENT_PROTOCOL_SCHEMA_VERSION, AgentStopFields, AgentStopOutcome, AgentStopRequest,
    AgentStopResult,
};
use yolo_hub_agent_run_protocol::{
    AGENT_RUN_PROTOCOL_SCHEMA_VERSION, AgentRunSelector, AgentRunState, GetAgentRunFields,
    GetAgentRunRequest,
};
use yolo_hub_rpc_protocol::{Command, Reply, Response};
use yolo_hub_workspace_protocol::{
    GetWorkspaceFields, GetWorkspaceRequest, WORKSPACE_PROTOCOL_SCHEMA_VERSION, WorkspaceSelector,
};

#[test]
fn workspace_reply_accepts_the_case_mapping_used_by_the_store() {
    let mut workspace = workspace_summary();
    workspace.name = "Équipe".into();
    let request = request(Command::GetWorkspace {
        request: GetWorkspaceRequest::new(GetWorkspaceFields {
            schema_version: WORKSPACE_PROTOCOL_SCHEMA_VERSION,
            selector: WorkspaceSelector::Name("équipe".into()),
        })
        .unwrap(),
    });
    Response::new(
        &request,
        workspace.address.hub_id,
        Reply::Workspace { workspace },
    )
    .validate_for(&request)
    .unwrap();
}

#[test]
fn run_lookup_and_stop_correlate_names_and_aliases_with_the_store() {
    let mut run = agent_run_summary();
    run.name = "Équipe".into();
    run.aliases.push("BÜRO".into());
    let hub = run.address.hub_id;
    let get = request(Command::GetAgentRun {
        request: GetAgentRunRequest::new(GetAgentRunFields {
            schema_version: AGENT_RUN_PROTOCOL_SCHEMA_VERSION,
            selector: AgentRunSelector::Name("équipe".into()),
        })
        .unwrap(),
    });
    Response::new(&get, hub, Reply::AgentRun { run: run.clone() })
        .validate_for(&get)
        .unwrap();
    let stop = request(Command::AgentStop {
        request: AgentStopRequest::new(AgentStopFields {
            schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
            request_id: uuid::Uuid::new_v4(),
            selector: AgentRunSelector::Name("büro".into()),
            force: false,
        })
        .unwrap(),
    });
    run.state = AgentRunState::Stopped;
    let stopped = Reply::AgentStop {
        stop: AgentStopResult {
            run,
            outcome: AgentStopOutcome::Terminated,
        },
    };
    Response::new(&stop, hub, stopped)
        .validate_for(&stop)
        .unwrap();
}
