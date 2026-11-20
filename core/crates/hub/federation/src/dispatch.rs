use crate::forward_diagnostics::ForwardStage;
use crate::{Federation, node::now_ms, remote_source::RemoteSource};
use std::sync::atomic::Ordering;
use yolo_hub_agent_protocol::{
    AgentPromptOutcome, AgentPromptResult, AgentPromptWait, CommandOutcome,
};
use yolo_hub_dispatch::{Dispatcher, Operation, OperationFuture};
use yolo_hub_identity::HubId;
use yolo_hub_peer_protocol::{Call, MAX_CALL_LIFETIME_MS, Purpose, Signed, required_rights};
use yolo_hub_rpc_protocol::{
    Command, HUB_PROTOCOL_VERSION, Hello, MAX_CATALOG_PAGE_AGENTS, MAX_CATALOG_PAGE_TERMINALS,
    Reply, Request, SUPPORTED_METHODS,
};
use yolo_hub_terminal_protocol::PublicError;

impl Dispatcher for Federation {
    fn hub_id(&self) -> HubId {
        self.hub_id()
    }
    fn dispatch(&self, request: Request) -> OperationFuture<'_> {
        Box::pin(async move {
            let failed = |stage, code| {
                self.forward_failure(&request, self.hub_id(), None, None, stage, code)
            };
            self.state
                .verify()
                .map_err(|_| failed(ForwardStage::EntryState, PublicError::Unreachable))?;
            if self.closing.load(Ordering::Acquire) {
                return Err(failed(ForwardStage::EntryRetired, PublicError::Unreachable));
            }
            if request.fields().command.local_only() {
                return Err(PublicError::Unsupported);
            }
            if request.fields().destination == self.hub_id() {
                return self.local_dispatch(request).await;
            }
            let now = now_ms().map_err(|_| PublicError::Internal)?;
            let origin = request.fields().destination;
            let required =
                required_rights(&request.fields().command).ok_or(PublicError::Unsupported)?;
            let authority = self
                .book
                .lock()
                .await
                .route_for(origin, &required, now)
                .map_err(|_| failed(ForwardStage::RouteLookup, PublicError::Internal))?
                .ok_or_else(|| failed(ForwardStage::MissingRoute, PublicError::Unreachable))?
                .authority
                .clone();
            let call = Signed::new(
                Call {
                    request,
                    authority,
                    issued_at_ms: now,
                    expires_at_ms: now
                        .checked_add(MAX_CALL_LIFETIME_MS)
                        .ok_or(PublicError::Internal)?,
                },
                Purpose::Call,
                self.state.as_ref(),
            )
            .map_err(|_| PublicError::Internal)?;
            let (response, channel) = self.forward(&call).await?;
            self.admit_entry_read_reply(&call).await?;
            match response.payload.response.reply {
                Reply::Attached { attachment } => {
                    Ok(Operation::Attached(Box::new(RemoteSource::new(
                        attachment,
                        channel.ok_or(PublicError::InvalidPayload)?,
                        call,
                    ))))
                }
                reply => Ok(Operation::Reply(reply)),
            }
        })
    }
}
impl Federation {
    async fn local_dispatch(&self, request: Request) -> Result<Operation, PublicError> {
        let now = now_ms().map_err(|_| PublicError::Internal)?;
        match &request.fields().command {
            Command::Hello {} => Ok(Operation::Reply(Reply::Hello {
                hello: Hello {
                    hub_id: self.hub_id(),
                    version: HUB_PROTOCOL_VERSION,
                    methods: SUPPORTED_METHODS.to_vec(),
                },
            })),
            Command::ListHubs { after } => {
                let book = self.book.lock().await;
                crate::hub_inventory::list_hubs(
                    self.hub_id(),
                    &book,
                    now,
                    *after,
                    &crate::hub_inventory::labels(&self.config),
                )
                .map(Operation::Reply)
            }
            Command::List {
                include_archived,
                after,
            } => {
                let local = self.backend.terminals(*include_archived).await.unwrap_or_else(|_| {
                    let mut terminals = self
                        .source()
                        .map(|source| source.payload.terminals.clone())
                        .unwrap_or_default();
                    for session in &mut terminals {
                        session.mark_unreachable();
                    }
                    terminals
                });
                let book = self.book.lock().await;
                let catalogues = book.observations(now).map_err(|_| PublicError::Internal)?;
                let (terminals, next) = yolo_hub_federation_aggregate::terminals(
                    local,
                    &catalogues,
                    *include_archived,
                    *after,
                    MAX_CATALOG_PAGE_TERMINALS,
                );
                Ok(Operation::Reply(Reply::Terminals { terminals, next }))
            }
            Command::ListAgents { after } => {
                let local = self.backend.observe_agents().await?;
                let book = self.book.lock().await;
                let catalogues = book.observations(now).map_err(|_| PublicError::Internal)?;
                let (agents, next) = yolo_hub_federation_aggregate::agents(
                    local,
                    &catalogues,
                    *after,
                    MAX_CATALOG_PAGE_AGENTS,
                );
                Ok(Operation::Reply(Reply::Agents { agents, next }))
            }
            Command::ListAgentRuns { request } => {
                let local = self.backend.platform_runs(request).await?;
                let book = self.book.lock().await;
                let catalogues = book.observations(now).map_err(|_| PublicError::Internal)?;
                Ok(Operation::Reply(Reply::AgentRuns {
                    list: yolo_hub_federation_aggregate::runs(request, local, &catalogues),
                }))
            }
            Command::ListWorkspaces { request } => {
                let local = self.backend.platform_workspaces(request).await?;
                let book = self.book.lock().await;
                let catalogues = book.observations(now).map_err(|_| PublicError::Internal)?;
                Ok(Operation::Reply(Reply::Workspaces {
                    list: yolo_hub_federation_aggregate::workspaces(request, local, &catalogues),
                }))
            }
            Command::AgentPrompt { request } => {
                let execution = self.backend.prompt_agent(request).await?;
                let (outcome, snapshot) = match execution.receipt.outcome() {
                    CommandOutcome::Failed { .. } => (AgentPromptOutcome::Rejected, None),
                    CommandOutcome::Unknown {} => (AgentPromptOutcome::Unknown, None),
                    _ => match request.fields().wait {
                        AgentPromptWait::None => (AgentPromptOutcome::Delivered, None),
                        AgentPromptWait::TurnEnd { timeout_ms } => yolo_hub_agent_wait::prompt_turn(
                            &self.backend, self.hub_id(), execution.target, execution.baseline,
                            timeout_ms, self.catalogue_wake.subscribe(),
                        )
                        .await?,
                    },
                };
                Ok(Operation::Reply(Reply::AgentPrompt {
                    prompt: AgentPromptResult {
                        receipt: execution.receipt,
                        terminal: execution.terminal,
                        outcome,
                        snapshot,
                    },
                }))
            }
            Command::WaitAgents { request } => {
                let mut request = request.clone();
                self.backend.expand_wait_runs(&mut request).await?;
                let deadline = tokio::time::Instant::now()
                    + std::time::Duration::from_millis(u64::from(request.wait_ms));
                let result = yolo_hub_agent_wait::wait(
                    &self.backend,
                    self.hub_id(),
                    &request,
                    self.catalogue_wake.subscribe(),
                    deadline,
                )
                .await?;
                Ok(Operation::Reply(Reply::AgentWait { wait: result }))
            }
            _ => self.backend.dispatch(request).await,
        }
    }
}
