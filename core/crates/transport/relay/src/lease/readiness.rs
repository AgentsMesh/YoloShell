use super::LeaseController;
use crate::relay::RelayStatus;

impl LeaseController {
    pub(crate) fn control_ready(&self) -> bool {
        !self.is_terminal()
            && self
                .generation()
                .is_some_and(|generation| self.prepared_generation == Some(generation))
            && !self.maintenance_due()
    }

    pub(crate) fn mark_generation_prepared(&mut self, generation: u64) -> bool {
        if self.is_terminal() || self.generation() != Some(generation) {
            return false;
        }
        self.prepared_generation = Some(generation);
        true
    }

    pub(crate) fn announce_control_ready(&mut self) {
        if !self.is_terminal() && self.control_ready() && !self.status_announced {
            self.status.send(RelayStatus::ControlAcquired);
            self.status_announced = true;
        }
    }
}
