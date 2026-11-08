use crate::{
    commands::*,
    fixtures::{constants::*, *},
};
use yolo_hub_agent_run_protocol::ListAgentRunsResult;
use yolo_hub_identity::AgentRunId;
use yolo_hub_rpc_protocol::{Command, Reply, Response};
use yolo_hub_terminal_protocol::PublicError;
use yolo_hub_workspace_protocol::ListWorkspacesResult;

fn foreign_run() -> yolo_hub_agent_run_protocol::AgentRunSummary {
    let mut run = agent_run_summary();
    run.address.hub_id = hub(SECOND_HUB_ID);
    run.address.run_id = AgentRunId::from_uuid(id(SECOND_SESSION_ID)).unwrap();
    run
}

fn foreign_workspace() -> yolo_hub_workspace_protocol::WorkspaceSummary {
    let mut workspace = workspace_summary();
    workspace.address.hub_id = hub(SECOND_HUB_ID);
    workspace
}

#[test]
fn aggregated_lists_accept_foreign_origins_but_reject_disorder() {
    let run_request = request(Command::ListAgentRuns {
        request: list_agent_runs(),
    });
    let runs = vec![agent_run_summary(), foreign_run()];
    let sorted = Reply::AgentRuns {
        list: ListAgentRunsResult { runs, next: None },
    };
    Response::new(&run_request, hub(HUB_ID), sorted)
        .validate_for(&run_request)
        .unwrap();
    let unsorted = Reply::AgentRuns {
        list: ListAgentRunsResult {
            runs: vec![foreign_run(), agent_run_summary()],
            next: None,
        },
    };
    assert_eq!(
        Response::new(&run_request, hub(HUB_ID), unsorted).validate_for(&run_request),
        Err(PublicError::InvalidPayload)
    );

    let workspace_request = request(Command::ListWorkspaces {
        request: list_workspaces(),
    });
    let sorted = Reply::Workspaces {
        list: ListWorkspacesResult {
            workspaces: vec![workspace_summary(), foreign_workspace()],
            next: None,
        },
    };
    Response::new(&workspace_request, hub(HUB_ID), sorted)
        .validate_for(&workspace_request)
        .unwrap();

    let worktree_request = request(Command::ListWorktrees {
        request: list_worktrees(),
    });
    let mut worktree = worktree_summary();
    worktree.address.hub_id = hub(SECOND_HUB_ID);
    let foreign = Reply::Workspaces {
        list: ListWorkspacesResult {
            workspaces: vec![worktree],
            next: None,
        },
    };
    assert_eq!(
        Response::new(&worktree_request, hub(HUB_ID), foreign).validate_for(&worktree_request),
        Err(PublicError::InvalidPayload)
    );
}
