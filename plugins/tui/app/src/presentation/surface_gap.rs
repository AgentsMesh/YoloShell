use crate::State;
use yolo_hub_identity::TerminalAddress;

impl State {
    /// Tears down a surface whose output stream can no longer be trusted.
    /// Every gap detection path uses this transition so the same facts survive.
    pub(crate) fn surface_gap(&mut self, address: TerminalAddress, message: Option<String>) {
        if let Some(message) = message {
            self.status = message;
        }
        self.clear_geometry_syncing(address);
        self.mark_resync(address);
        self.retire_query_scope(address);
        self.clear_offline(address);
        self.surfaces.remove(&address);
    }
}
