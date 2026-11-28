use crate::LocalBackend;
use yolo_agent_control_contract::NativeControlObservation;
use yolo_hub_agent_observe::{ObservationStage, classify};
use yolo_terminal_model::TerminalView;

impl LocalBackend {
    pub(crate) async fn capture_native_observation(
        &self,
        view: &TerminalView,
    ) -> Option<NativeControlObservation> {
        classify(&self.client, view).await.ok()
    }

    pub(crate) async fn classify_native_observation(
        &self,
        view: &TerminalView,
    ) -> Result<NativeControlObservation, ObservationStage> {
        classify(&self.client, view).await
    }
}
