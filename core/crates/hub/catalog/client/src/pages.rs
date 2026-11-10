use crate::{CatalogError, constants::MAX_CATALOG_PAGES};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    time::Instant,
};
use yolo_hub_agent_protocol::AgentSnapshot;
use yolo_hub_client::Client;
use yolo_hub_identity::{AgentTarget, HubId, TerminalAddress};
use yolo_hub_rpc_protocol::HubSummary;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::TerminalSummary;

pub(crate) async fn collect<S, T, K>(
    client: &mut Client<S>,
    deadline: Instant,
    command: impl Fn(Option<K>) -> Command,
    extract: impl Fn(Reply) -> Option<(Vec<T>, Option<K>)>,
) -> Result<Vec<T>, CatalogError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let hub = client.hello().hub_id;
    let mut values = Vec::new();
    let mut after = None;
    for _ in 0..MAX_CATALOG_PAGES {
        let reply = client.call(hub, command(after), deadline).await?;
        let (page, next) = extract(reply).ok_or(CatalogError::UnexpectedReply)?;
        values.extend(page);
        if next.is_none() {
            return Ok(values);
        }
        after = next;
    }
    Err(CatalogError::Limit)
}

pub(crate) fn terminals(reply: Reply) -> Option<(Vec<TerminalSummary>, Option<TerminalAddress>)> {
    match reply {
        Reply::Terminals { terminals, next } => Some((terminals, next)),
        _ => None,
    }
}
pub(crate) fn agents(reply: Reply) -> Option<(Vec<AgentSnapshot>, Option<AgentTarget>)> {
    match reply {
        Reply::Agents { agents, next } => Some((agents, next)),
        _ => None,
    }
}
pub(crate) fn hubs(reply: Reply) -> Option<(Vec<HubSummary>, Option<HubId>)> {
    match reply {
        Reply::Hubs { hubs, next } => Some((hubs, next)),
        _ => None,
    }
}
