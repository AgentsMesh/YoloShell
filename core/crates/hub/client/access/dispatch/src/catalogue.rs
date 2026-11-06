use crate::ScopedDispatcher;
use yolo_hub_rpc_protocol::{
    Command, MAX_CATALOG_PAGE_AGENTS, MAX_CATALOG_PAGE_HUBS, MAX_CATALOG_PAGE_TERMINALS, Reply,
    Request,
};
use yolo_hub_terminal_protocol::PublicError;

impl ScopedDispatcher {
    pub(crate) async fn catalogue(&self, request: &Request) -> Result<Option<Reply>, PublicError> {
        Ok(Some(match request.fields().command {
            Command::List {
                include_archived,
                after,
            } => {
                let (terminals, next) = self
                    .page(
                        after,
                        MAX_CATALOG_PAGE_TERMINALS,
                        |after| Command::List {
                            include_archived,
                            after,
                        },
                        |reply| match reply {
                            Reply::Terminals { terminals, next } => Ok((terminals, next)),
                            _ => Err(PublicError::InvalidPayload),
                        },
                        |session| session.fields().address,
                        |session| session.fields().address.hub_id,
                    )
                    .await?;
                Reply::Terminals { terminals, next }
            }
            Command::ListAgents { after } => {
                let (agents, next) = self
                    .page(
                        after,
                        MAX_CATALOG_PAGE_AGENTS,
                        |after| Command::ListAgents { after },
                        |reply| match reply {
                            Reply::Agents { agents, next } => Ok((agents, next)),
                            _ => Err(PublicError::InvalidPayload),
                        },
                        |agent| agent.cursor().target(),
                        |agent| agent.cursor().target().address.hub_id,
                    )
                    .await?;
                Reply::Agents { agents, next }
            }
            Command::ListHubs { after } => {
                let (hubs, next) = self
                    .page(
                        after,
                        MAX_CATALOG_PAGE_HUBS,
                        |after| Command::ListHubs { after },
                        |reply| match reply {
                            Reply::Hubs { hubs, next } => Ok((hubs, next)),
                            _ => Err(PublicError::InvalidPayload),
                        },
                        |hub| hub.hub_id,
                        |hub| hub.hub_id,
                    )
                    .await?;
                Reply::Hubs { hubs, next }
            }
            _ => return Ok(None),
        }))
    }
}
