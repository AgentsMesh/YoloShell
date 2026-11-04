use crate::workspace::WorkspaceService;
use yolo_hub_terminal_protocol::PublicError;
use yolo_hub_workspace_protocol::WorkspaceAddress;
use yolo_hub_workspace_store::WorkspaceReservation;

impl WorkspaceService {
    /// Holds removal off until the launch has bound its terminal or failed.
    pub(crate) async fn reserve_creation(
        &self,
        address: WorkspaceAddress,
    ) -> Result<WorkspaceReservation, PublicError> {
        self.store()
            .lock()
            .await
            .reserve_creation(address)
            .map_err(yolo_hub_workspace_store::StoreError::into_public)
    }
}
