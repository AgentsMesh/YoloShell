mod constants;
mod pages;

use tokio::{
    io::{AsyncRead, AsyncWrite},
    time::Instant,
};
use yolo_hub_agent_protocol::AgentSnapshot;
use yolo_hub_agent_protocol::{Freshness, Lifecycle, Reachability};
use yolo_hub_client::{Client, ClientError};
use yolo_hub_rpc_protocol::{Command, HubSummary, Method};
use yolo_hub_terminal_protocol::{RuntimeStatus, TerminalSummary};

#[derive(Clone, Debug, Default)]
pub struct Catalog {
    pub terminals: Vec<TerminalSummary>,
    pub agents: Option<Vec<AgentSnapshot>>,
    pub hubs: Option<Vec<HubSummary>>,
}

impl Catalog {
    /// A current catalogue is actionable only when every live AgentRun has the
    /// exact Terminal binding observed in the same refresh.
    pub fn is_coherent(&self) -> bool {
        self.agents.as_ref().is_none_or(|agents| {
            agents.iter().filter(|agent| current(agent)).all(|agent| {
                let target = agent.cursor().target();
                self.terminals.iter().any(|terminal| {
                    let fields = terminal.fields();
                    fields.address == agent.fields().terminal.address
                        && fields.agent == Some(target)
                        && fields.runtime.is_some_and(|runtime| {
                            runtime.target == agent.fields().terminal
                                && !matches!(
                                    runtime.status,
                                    RuntimeStatus::Exited | RuntimeStatus::Lost
                                )
                        })
                })
            })
        })
    }
}

fn current(agent: &AgentSnapshot) -> bool {
    let fields = agent.fields();
    !matches!(fields.lifecycle, Lifecycle::Exited { .. })
        && fields.freshness == Freshness::Current
        && fields.reachability == Reachability::Reachable
}

#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    #[error(transparent)]
    Client(#[from] ClientError),
    #[error("Hub catalog exceeds the client page limit")]
    Limit,
    #[error("Unexpected Hub catalog response")]
    UnexpectedReply,
}

/// Read one complete refresh without publishing partial results or retrying requests.
/// The collections are not a cross-origin transaction; consumers must fence bindings.
///
/// Archived terminal records stay outside the default catalogue. Callers that
/// expose history or recovery use [`read_with_archived`] so the record remains
/// available to their liveness projection instead of disappearing.
pub async fn read<S: AsyncRead + AsyncWrite + Unpin>(
    client: &mut Client<S>,
    deadline: Instant,
) -> Result<Catalog, CatalogError> {
    read_with_archived(client, deadline, false).await
}

pub async fn read_with_archived<S: AsyncRead + AsyncWrite + Unpin>(
    client: &mut Client<S>,
    deadline: Instant,
    include_archived: bool,
) -> Result<Catalog, CatalogError> {
    let terminals = pages::collect(
        client,
        deadline,
        |after| Command::List {
            include_archived,
            after,
        },
        pages::terminals,
    )
    .await?;
    let agents = if client.hello().methods.contains(&Method::ListAgents) {
        Some(
            pages::collect(
                client,
                deadline,
                |after| Command::ListAgents { after },
                pages::agents,
            )
            .await?,
        )
    } else {
        None
    };
    let hubs = if client.hello().methods.contains(&Method::ListHubs) {
        Some(
            pages::collect(
                client,
                deadline,
                |after| Command::ListHubs { after },
                pages::hubs,
            )
            .await?,
        )
    } else {
        None
    };
    Ok(Catalog {
        terminals,
        agents,
        hubs,
    })
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod tests;
