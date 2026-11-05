use super::{CapabilityScanResult, PtyCapabilityScanState};
use crate::process::inventory_stable::StableInventory;
use std::sync::Arc;

impl PtyCapabilityScanState {
    pub(in crate::process) fn frozen_inventory(&self) -> Option<Arc<StableInventory>> {
        self.released.frozen_inventory()
    }

    pub(in crate::process) fn invalidate_frozen_inventory(&mut self) {
        self.released.clear_frozen_inventory();
    }

    #[cfg(test)]
    pub(in crate::process) fn install_frozen_inventory_for_test(
        &mut self,
        inventory: Arc<StableInventory>,
    ) {
        self.released.install_frozen_inventory_for_test(inventory);
    }

    #[cfg(test)]
    pub(in crate::process) fn has_frozen_inventory_for_test(&self) -> bool {
        self.frozen_inventory().is_some()
    }
}

impl CapabilityScanResult {
    pub(in crate::process) fn retain_frozen_inventory(&mut self, inventory: Arc<StableInventory>) {
        self.advance
            .next
            .released
            .retain_frozen_inventory(inventory);
    }
}
