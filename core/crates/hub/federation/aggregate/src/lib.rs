//! Read aggregation for an entry Hub: merge local pages with each origin's
//! published peer catalogue. Ordering and cursor semantics stay identical to a
//! single Hub so a client can page through the union without gaps.

use yolo_hub_agent_protocol::AgentSnapshot;
use yolo_hub_agent_run_protocol::{ListAgentRunsRequest, ListAgentRunsResult};
use yolo_hub_identity::{AgentTarget, TerminalAddress};
use yolo_hub_peer_protocol::Catalogue;
use yolo_hub_terminal_protocol::TerminalSummary;
use yolo_hub_workspace_protocol::{ListWorkspacesRequest, ListWorkspacesResult};

pub fn terminals(
    local: Vec<TerminalSummary>,
    catalogues: &[Catalogue],
    include_archived: bool,
    after: Option<TerminalAddress>,
    limit: usize,
) -> (Vec<TerminalSummary>, Option<TerminalAddress>) {
    let mut terminals = local;
    for catalogue in catalogues {
        terminals.extend(catalogue.terminals.iter().cloned());
    }
    terminals.retain(|session| {
        (include_archived || !session.fields().archived)
            && after.is_none_or(|after| session.fields().address > after)
    });
    terminals.sort_by_key(|session| session.fields().address);
    terminals.dedup_by_key(|session| session.fields().address);
    let more = terminals.len() > limit;
    terminals.truncate(limit);
    let next = more
        .then(|| terminals.last().map(|session| session.fields().address))
        .flatten();
    (terminals, next)
}

pub fn agents(
    local: Vec<AgentSnapshot>,
    catalogues: &[Catalogue],
    after: Option<AgentTarget>,
    limit: usize,
) -> (Vec<AgentSnapshot>, Option<AgentTarget>) {
    let mut agents = local;
    for catalogue in catalogues {
        agents.extend(catalogue.agents.iter().cloned());
    }
    agents.retain(|agent| after.is_none_or(|after| agent.cursor().target() > after));
    agents.sort_by_key(|agent| agent.cursor().target());
    agents.dedup_by_key(|agent| agent.cursor().target());
    let more = agents.len() > limit;
    agents.truncate(limit);
    let next = more
        .then(|| agents.last().map(|agent| agent.cursor().target()))
        .flatten();
    (agents, next)
}

pub fn runs(
    request: &ListAgentRunsRequest,
    local: ListAgentRunsResult,
    catalogues: &[Catalogue],
) -> ListAgentRunsResult {
    let fields = request.fields();
    let local_more = local.next.is_some();
    let mut runs = local.runs;
    for catalogue in catalogues {
        runs.extend(catalogue.runs.iter().cloned());
    }
    runs.retain(|run| {
        fields.after.is_none_or(|after| run.address > after)
            && fields
                .filter
                .as_ref()
                .is_none_or(|filter| filter.matches(run))
    });
    runs.sort_by_key(|run| run.address);
    runs.dedup_by_key(|run| run.address);
    let limit = fields.limit as usize;
    let more = runs.len() > limit || local_more;
    runs.truncate(limit);
    let next = more
        .then(|| runs.last().map(|run| run.address))
        .flatten();
    ListAgentRunsResult { runs, next }
}

pub fn workspaces(
    request: &ListWorkspacesRequest,
    local: ListWorkspacesResult,
    catalogues: &[Catalogue],
) -> ListWorkspacesResult {
    let fields = request.fields();
    let local_more = local.next.is_some();
    let mut workspaces = local.workspaces;
    for catalogue in catalogues {
        workspaces.extend(catalogue.workspaces.iter().cloned());
    }
    workspaces.retain(|workspace| {
        fields
            .after
            .is_none_or(|after| workspace.address > after)
    });
    workspaces.sort_by_key(|workspace| workspace.address);
    workspaces.dedup_by_key(|workspace| workspace.address);
    let limit = fields.limit as usize;
    let more = workspaces.len() > limit || local_more;
    workspaces.truncate(limit);
    let next = more
        .then(|| workspaces.last().map(|workspace| workspace.address))
        .flatten();
    ListWorkspacesResult { workspaces, next }
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
