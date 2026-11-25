use std::sync::Arc;
use yolo_hub_dispatch::{Dispatcher, OriginDecorator};
use yolo_hub_identity::HubId;
use yolo_hub_local_backend::LocalBackend;
use yolo_hub_local_providers::ProviderDispatcher;
use yolo_hub_local_terminal_history::HistoryDispatcher;
use yolo_hub_rpc_protocol::ProviderSummary;

/// Origin services decorate the local backend for calls entering through a
/// peer link: terminal history and provider control share the local Host.
pub(crate) struct OriginServices {
    pub(crate) hub: HubId,
    pub(crate) host: yolo_client::Client,
    pub(crate) providers: Vec<ProviderSummary>,
}

impl OriginDecorator<LocalBackend> for OriginServices {
    fn decorate(&self, backend: LocalBackend) -> Arc<dyn Dispatcher> {
        let history = Arc::new(HistoryDispatcher::new(
            Arc::new(backend),
            self.hub,
            self.host.clone(),
        ));
        Arc::new(ProviderDispatcher::new(
            history,
            self.hub,
            self.host.clone(),
            self.providers.clone(),
        ))
    }
}
