use yolo_hub_agent_run_protocol::{
    AGENT_RUN_PROTOCOL_SCHEMA_VERSION, AgentRunAddress, AgentRunSelector, AgentRunSummary,
    GetAgentRunRequest, ListAgentRunsFields, ListAgentRunsRequest, ListAgentRunsResult,
    UpdateAgentRunRequest,
};
use yolo_hub_agent_run_store::{AgentRunStore, ReserveAgentRun, StoreError};
use yolo_hub_identity::{HubId, TerminalAddress, WorkspaceAddress};
use yolo_hub_terminal_protocol::PublicError;
use yolo_paths::directory::PrivateDirectory;

/// Hub-owned durable run identity. The store owns naming and revisions; the
/// terminal and provider binding remain observed Host/Worker facts.
pub struct AgentRunService {
    store: tokio::sync::Mutex<AgentRunStore>,
}

impl AgentRunService {
    pub fn shared(
        directory: PrivateDirectory,
        hub: HubId,
    ) -> Result<std::sync::Arc<Self>, PublicError> {
        Ok(std::sync::Arc::new(Self::open(directory, hub)?))
    }

    pub fn open(directory: PrivateDirectory, hub: HubId) -> Result<Self, PublicError> {
        Ok(Self {
            store: tokio::sync::Mutex::new(
                AgentRunStore::open(directory, hub).map_err(StoreError::into_public)?,
            ),
        })
    }

    pub async fn resume(
        &self,
        start_token: uuid::Uuid,
    ) -> Result<Option<AgentRunSummary>, PublicError> {
        self.store
            .lock()
            .await
            .resume(start_token)
            .map_err(StoreError::into_public)
    }

    /// Adopt observed executions that have no durable run. Conflicts, invalid
    /// providers and quota exhaustion are skipped by the store.
    pub async fn adopt_observed(&self, observed: &[(TerminalAddress, String)]) {
        let mut store = self.store.lock().await;
        for (terminal, provider_id) in observed {
            let _ = store.adopt(*terminal, provider_id);
        }
    }

    pub async fn reserve(
        &self,
        run_id: uuid::Uuid,
        name: String,
        provider_id: String,
        workspace: Option<WorkspaceAddress>,
        parent: Option<AgentRunAddress>,
        start_token: uuid::Uuid,
    ) -> Result<AgentRunSummary, PublicError> {
        self.store
            .lock()
            .await
            .reserve(ReserveAgentRun {
                run_id,
                name,
                provider_id,
                workspace,
                parent,
                start_token,
            })
            .map_err(StoreError::into_public)
    }

    pub async fn rejoin_starting(&self, address: AgentRunAddress) -> Result<(), PublicError> {
        self.store
            .lock()
            .await
            .rejoin_starting(address)
            .map_err(StoreError::into_public)
    }

    pub async fn cancel_starting(
        &self,
        address: AgentRunAddress,
    ) -> Result<AgentRunSummary, PublicError> {
        self.store
            .lock()
            .await
            .cancel_starting(address)
            .map_err(StoreError::into_public)
    }

    pub async fn mark_launched(
        &self,
        address: AgentRunAddress,
        terminal: TerminalAddress,
    ) -> Result<AgentRunSummary, PublicError> {
        self.store
            .lock()
            .await
            .mark_launched(address, terminal)
            .map_err(StoreError::into_public)
    }

    pub(crate) async fn get(&self, request: &GetAgentRunRequest) -> Result<AgentRunSummary, PublicError> {
        self.store
            .lock()
            .await
            .get(request)
            .map_err(StoreError::into_public)?
            .ok_or(PublicError::NotFound)
    }

    pub async fn resolve(
        &self,
        selector: &AgentRunSelector,
    ) -> Result<AgentRunSummary, PublicError> {
        self.store
            .lock()
            .await
            .find_selector(selector)
            .map_err(StoreError::into_public)?
            .ok_or(PublicError::NotFound)
    }

    pub async fn mark_stopped(
        &self,
        address: AgentRunAddress,
    ) -> Result<AgentRunSummary, PublicError> {
        self.store
            .lock()
            .await
            .mark_stopped(address)
            .map_err(StoreError::into_public)
    }

    pub async fn snapshot_page(
        &self,
        limit: u16,
    ) -> Result<(Vec<AgentRunSummary>, bool), PublicError> {
        let request = ListAgentRunsRequest::new(ListAgentRunsFields {
            schema_version: AGENT_RUN_PROTOCOL_SCHEMA_VERSION,
            after: None,
            limit,
            filter: None,
        })?;
        let (runs, next) = self
            .store
            .lock()
            .await
            .list(&request)
            .map_err(StoreError::into_public)?;
        Ok((runs, next.is_some()))
    }

    pub async fn list(
        &self,
        request: &ListAgentRunsRequest,
    ) -> Result<ListAgentRunsResult, PublicError> {
        let (runs, next) = self
            .store
            .lock()
            .await
            .list(request)
            .map_err(StoreError::into_public)?;
        Ok(ListAgentRunsResult { runs, next })
    }

    pub(crate) async fn update(
        &self,
        request: &UpdateAgentRunRequest,
    ) -> Result<AgentRunSummary, PublicError> {
        self.store
            .lock()
            .await
            .update(request)
            .map_err(StoreError::into_public)
    }
}

pub fn run_name(provider_id: &str, start_token: uuid::Uuid) -> String {
    let suffix = &start_token.simple().to_string()[..8];
    format!("{provider_id}-{suffix}")
}

pub fn selector_address(selector: &AgentRunSelector) -> Option<AgentRunAddress> {
    match selector {
        AgentRunSelector::Address(address) => Some(*address),
        AgentRunSelector::Name(_) => None,
    }
}
