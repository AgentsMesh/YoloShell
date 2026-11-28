use crate::LocalBackend;
use tokio::sync::MutexGuard;

impl LocalBackend {
    /// Order: native projection (when used), observation gate, registry. Hold the
    /// gate across Host reads and reconciliation so an older catalogue cannot
    /// supersede a registration or deactivate a generation admitted afterward.
    /// The registry never acquires the gate; cancellation drops the guard.
    pub(crate) async fn connector_observation(&self) -> Option<MutexGuard<'_, ()>> {
        if self.connectors.is_some() {
            Some(self.connector_observation_gate.lock().await)
        } else {
            None
        }
    }
}
