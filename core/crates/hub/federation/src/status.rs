use crate::Federation;
use yolo_hub_identity::HubId;

impl Federation {
    /// Currently open directed links, for diagnostics and acceptance checks.
    pub async fn connected_peers(&self) -> Vec<HubId> {
        self.connections
            .read()
            .await
            .iter()
            .filter(|(_, link)| !link.is_closed())
            .map(|(hub, _)| *hub)
            .collect()
    }
}
