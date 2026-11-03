use crate::constants::*;
use crate::reject::rejected;
use anyhow::Result;
use std::collections::BTreeMap;
use std::str::FromStr;
use tokio::time::Instant;
use yolo_hub_agent_protocol::{AgentSnapshot, Freshness, Lifecycle, Reachability};
use yolo_hub_catalog_client::{Catalog, read_with_archived};
use yolo_hub_client::LocalClient;
use yolo_hub_identity::{AgentId, AgentTarget, TerminalAddress, TerminalId};
use yolo_hub_terminal_protocol::TerminalSummary;

pub struct AgentRow {
    pub terminal_name: Option<String>,
    pub snapshot: AgentSnapshot,
}

pub struct Selection {
    pub target: AgentTarget,
    pub terminal_name: Option<String>,
}

/// Archived terminals are history. Only an explicit `--all` listing needs
/// them; every other listing scopes the Host read to live sessions.
async fn catalog(client: &mut LocalClient, include_archived: bool) -> Result<Catalog> {
    let deadline = Instant::now() + RPC_TIMEOUT;
    Ok(read_with_archived(client, deadline, include_archived).await?)
}

pub async fn agents(client: &mut LocalClient, all: bool) -> Result<Vec<AgentRow>> {
    let catalog = catalog(client, all).await?;
    let agents = catalog
        .agents
        .ok_or_else(|| rejected(AGENT_LIST_UNSUPPORTED))?;
    let names: BTreeMap<TerminalAddress, String> = catalog
        .terminals
        .iter()
        .map(|terminal| (terminal.fields().address, terminal.fields().name.clone()))
        .collect();
    Ok(agents
        .into_iter()
        .filter(|snapshot| all || is_active(snapshot))
        .map(|snapshot| AgentRow {
            terminal_name: names.get(&snapshot.fields().terminal.address).cloned(),
            snapshot,
        })
        .collect())
}

fn is_active(snapshot: &AgentSnapshot) -> bool {
    let fields = snapshot.fields();
    !matches!(fields.lifecycle, Lifecycle::Exited { .. })
        && fields.freshness == Freshness::Current
        && fields.reachability == Reachability::Reachable
}

/// Resolve a selector against the complete catalogue, so a history query can
/// still name an archived terminal.
pub async fn resolve(client: &mut LocalClient, selector: &str) -> Result<Selection> {
    Ok(resolve_in(client, std::slice::from_ref(&selector.to_owned()), true)
        .await?
        .remove(0))
}

/// Control can only target a current execution, so its selector reads live
/// sessions only instead of paying for the archived history.
pub async fn resolve_live(client: &mut LocalClient, selector: &str) -> Result<Selection> {
    Ok(resolve_in(client, std::slice::from_ref(&selector.to_owned()), false)
        .await?
        .remove(0))
}

/// Resolve several live selectors from one catalogue read, preserving order
/// and dropping duplicates. A wait can name any of the returned targets.
pub async fn resolve_live_many(
    client: &mut LocalClient,
    selectors: &[String],
) -> Result<Vec<Selection>> {
    resolve_in(client, selectors, false).await
}
async fn resolve_in(
    client: &mut LocalClient,
    selectors: &[String],
    include_archived: bool,
) -> Result<Vec<Selection>> {
    let catalog = catalog(client, include_archived).await?;
    let agents = catalog
        .agents
        .ok_or_else(|| rejected(AGENT_LIST_UNSUPPORTED))?;
    let mut selections: Vec<Selection> = Vec::with_capacity(selectors.len());
    for selector in selectors {
        let target = by_selector(selector, &agents, &catalog.terminals)?;
        if selections.iter().any(|selection| selection.target == target) {
            continue;
        }
        let terminal_name = catalog
            .terminals
            .iter()
            .find(|terminal| terminal.fields().agent == Some(target))
            .map(|terminal| terminal.fields().name.clone());
        selections.push(Selection {
            target,
            terminal_name,
        });
    }
    Ok(selections)
}

/// Agent and terminal ids are both UUIDs, so a terminal id also parses as an
/// agent id. Try the agent address first, then fall through to the terminal
/// address instead of rejecting the selector.
fn by_selector(
    selector: &str,
    agents: &[AgentSnapshot],
    terminals: &[TerminalSummary],
) -> Result<AgentTarget> {
    if let Ok(id) = AgentId::from_str(selector) {
        if let Some(target) = by_agent(agents, terminals, id)? {
            return Ok(target);
        }
    }
    if let Ok(id) = TerminalId::from_str(selector) {
        return by_terminal_id(terminals, id);
    }
    by_name(terminals, selector.trim())
}

fn by_agent(
    agents: &[AgentSnapshot],
    terminals: &[TerminalSummary],
    id: AgentId,
) -> Result<Option<AgentTarget>> {
    let matches: Vec<AgentTarget> = agents
        .iter()
        .map(|snapshot| snapshot.cursor().target())
        .filter(|target| target.address.agent_id == id)
        .collect();
    if matches.is_empty() {
        return Ok(None);
    }
    let current: Vec<AgentTarget> = matches
        .iter()
        .copied()
        .filter(|target| {
            terminals
                .iter()
                .any(|terminal| terminal.fields().agent == Some(*target))
        })
        .collect();
    match (current.len(), matches.len()) {
        (1, _) => Ok(Some(current[0])),
        (0, 1) => Ok(Some(matches[0])),
        _ => Err(rejected(SELECTOR_AMBIGUOUS)),
    }
}

fn by_terminal_id(terminals: &[TerminalSummary], id: TerminalId) -> Result<AgentTarget> {
    choose(
        terminals
            .iter()
            .filter(|terminal| terminal.fields().address.terminal_id == id)
            .collect(),
    )
}

fn by_name(terminals: &[TerminalSummary], name: &str) -> Result<AgentTarget> {
    choose(
        terminals
            .iter()
            .filter(|terminal| terminal.fields().name == name)
            .collect(),
    )
}

fn choose(candidates: Vec<&TerminalSummary>) -> Result<AgentTarget> {
    let active: Vec<&TerminalSummary> = candidates
        .iter()
        .copied()
        .filter(|terminal| !terminal.fields().archived)
        .collect();
    let selected = if active.len() == 1 {
        Some(active[0])
    } else if candidates.len() == 1 {
        Some(candidates[0])
    } else {
        None
    };
    let terminal = selected.ok_or_else(|| {
        rejected(if candidates.is_empty() {
            AGENT_NOT_FOUND
        } else {
            SELECTOR_AMBIGUOUS
        })
    })?;
    terminal
        .fields()
        .agent
        .ok_or_else(|| rejected(NO_CURRENT_AGENT))
}
