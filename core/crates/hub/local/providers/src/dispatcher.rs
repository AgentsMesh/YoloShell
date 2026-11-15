//! Provider availability projection over the Host that owns the launch
//! environment. This decorator sits inside client-access authorization and
//! delegates every unrelated command to the normal Hub dispatcher.

use std::collections::BTreeSet;
use std::sync::Arc;
use yolo_agent_contract::{PROVIDER_CAPABILITY_PREFIX, PROVIDER_CATALOG_CAPABILITY};
use yolo_hub_dispatch::{Dispatcher, Operation, OperationFuture};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, MAX_PROVIDERS, ProviderSummary, Reply, Request};
use yolo_hub_terminal_protocol::PublicError;

pub struct ProviderDispatcher {
    inner: Arc<dyn Dispatcher>,
    hub: HubId,
    client: yolo_client::Client,
    providers: Vec<ProviderSummary>,
}

impl ProviderDispatcher {
    pub fn new(
        inner: Arc<dyn Dispatcher>,
        hub: HubId,
        client: yolo_client::Client,
        providers: Vec<ProviderSummary>,
    ) -> Self {
        Self {
            inner,
            hub,
            client,
            providers,
        }
    }
}

impl Dispatcher for ProviderDispatcher {
    fn hub_id(&self) -> HubId {
        self.hub
    }

    fn dispatch(&self, request: Request) -> OperationFuture<'_> {
        Box::pin(async move {
            if request.fields().destination == self.hub {
                let provider_id = match &request.fields().command {
                    Command::ListProviders {} => {
                        let capabilities = self.capabilities().await?;
                        let providers = project(&self.providers, &capabilities)?;
                        return Ok(Operation::Reply(Reply::Providers { providers }));
                    }
                    Command::AgentStart { request } => Some(request.fields().provider_id.clone()),
                    _ => None,
                };
                if let Some(provider_id) = provider_id {
                    if provider_available(&self.capabilities().await?, &provider_id) == Some(false) {
                        return Err(PublicError::Unsupported);
                    }
                }
            }
            self.inner.dispatch(request).await
        })
    }
}

impl ProviderDispatcher {
    async fn capabilities(&self) -> Result<Vec<String>, PublicError> {
        self.client
            .status()
            .await
            .map(|status| status.capabilities)
            .map_err(|_| PublicError::Unreachable)
    }
}

pub(crate) fn project(
    providers: &[ProviderSummary],
    capabilities: &[String],
) -> Result<Vec<ProviderSummary>, PublicError> {
    if !capabilities.iter().any(|value| value == PROVIDER_CATALOG_CAPABILITY) {
        return Err(PublicError::Unsupported);
    }
    let available = capabilities
        .iter()
        .filter_map(|value| value.strip_prefix(PROVIDER_CAPABILITY_PREFIX))
        .collect::<BTreeSet<_>>();
    let mut values = providers
        .iter()
        .map(|provider| ProviderSummary {
            provider_id: provider.provider_id.clone(),
            display_name: provider.display_name.clone(),
            available: available.contains(provider.provider_id.as_str()),
            features: provider.features,
        })
        .collect::<Vec<_>>();
    values.sort_by(|left, right| left.provider_id.cmp(&right.provider_id));
    if values.len() > MAX_PROVIDERS {
        return Err(PublicError::Internal);
    }
    Ok(values)
}

/// `None` means the Host does not advertise provider facts at all.
pub(crate) fn provider_available(capabilities: &[String], provider_id: &str) -> Option<bool> {
    capabilities
        .iter()
        .any(|value| value == PROVIDER_CATALOG_CAPABILITY)
        .then(|| {
            capabilities
                .iter()
                .any(|value| value == &format!("{PROVIDER_CAPABILITY_PREFIX}{provider_id}"))
        })
}
