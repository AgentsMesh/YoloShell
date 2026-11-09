//! Bounded platform snapshot assembly. Terminal and agent evidence comes from
//! the local backend; runs and workspaces come from the platform services.

use crate::PlatformServices;
use crate::agent_run::AgentRunService;
use std::collections::{BTreeMap, BTreeSet};
use yolo_hub_agent_protocol::{AgentSnapshot, Freshness, Lifecycle};
use yolo_hub_agent_run_protocol::AgentRunSummary;
use yolo_hub_identity::{AgentTarget, HubId, TerminalAddress, TerminalId};
use yolo_hub_rpc_protocol::{
    GetPlatformSnapshotRequest, HubSummary, PLATFORM_SNAPSHOT_SCHEMA_VERSION, PlatformSnapshot,
    PlatformSnapshotFields, PlatformSnapshotTruncation, Reply,
};
use yolo_hub_terminal_protocol::{PublicError, TerminalSummary};

pub async fn platform_snapshot(
    services: Option<&PlatformServices>,
    hub: HubId,
    request: &GetPlatformSnapshotRequest,
    mut terminals: Vec<TerminalSummary>,
    mut agents: Vec<AgentSnapshot>,
) -> Result<Reply, PublicError> {
    let fields = request.fields();
    let observed: Vec<(TerminalId, AgentTarget)> = agents
        .iter()
        .map(|agent| {
            (
                agent.fields().terminal.address.terminal_id,
                agent.cursor().target(),
            )
        })
        .collect();
    terminals.sort_by_key(|terminal| terminal.fields().address);
    agents.sort_by_key(|agent| agent.cursor().target());
    let terminals_truncated = terminals.len() > fields.terminal_limit as usize;
    terminals.truncate(fields.terminal_limit as usize);
    let agents_truncated = agents.len() > fields.agent_limit as usize;
    agents.truncate(fields.agent_limit as usize);
    let (mut runs, runs_truncated) = match services {
        Some(services) => services.runs().snapshot_page(fields.run_limit).await?,
        None => (Vec::new(), false),
    };
    annotate_current(&mut runs, &observed);
    let (workspaces, workspaces_truncated) = match services {
        Some(services) => {
            services
                .workspaces()
                .snapshot_page(fields.workspace_limit)
                .await?
        }
        None => (Vec::new(), false),
    };
    let snapshot = PlatformSnapshot::new(PlatformSnapshotFields {
        schema_version: PLATFORM_SNAPSHOT_SCHEMA_VERSION,
        generated_at_ms: yolo_terminal_model::now_ms(),
        hubs: vec![HubSummary {
            hub_id: hub,
            reachable: true,
            label: None,
        }],
        terminals,
        agents,
        runs,
        workspaces,
        truncated: PlatformSnapshotTruncation {
            hubs: false,
            terminals: terminals_truncated,
            agents: agents_truncated,
            runs: runs_truncated,
            workspaces: workspaces_truncated,
        },
    })?;
    Ok(Reply::PlatformSnapshot { snapshot })
}

pub fn annotate_current(runs: &mut [AgentRunSummary], observed: &[(TerminalId, AgentTarget)]) {
    yolo_hub_agent_run_protocol::annotate_current(runs, observed);
}

/// Give an observed provider execution a durable run identity when it has none.
///
/// Only a current, non-exited execution is adopted, and every terminal yields
/// at most one run. Adoption is best effort: conflicts and quota exhaustion
/// leave the execution unadopted instead of failing the observation read.
pub async fn adopt_observed(runs: Option<&AgentRunService>, agents: &[AgentSnapshot]) {
    let Some(runs) = runs else {
        return;
    };
    let mut seen = BTreeSet::new();
    let observed: Vec<(TerminalAddress, String)> = agents
        .iter()
        .filter_map(|agent| {
            let fields = agent.fields();
            (fields.freshness == Freshness::Current
                && !matches!(fields.lifecycle, Lifecycle::Exited { .. })
                && seen.insert(fields.terminal.address))
            .then(|| (fields.terminal.address, fields.provider.kind().to_string()))
        })
        .collect();
    runs.adopt_observed(&observed).await;
}
