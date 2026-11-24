use super::RootWaitAuthority;
use crate::process::signal_authority::{
    ExactProcessSignal, request_waitable_root_graceful_termination, signal_exact_process,
};
use anyhow::Result;

impl RootWaitAuthority {
    pub fn request_graceful_termination(&self) -> Result<bool> {
        if self.waited_status.is_some() {
            return Ok(false);
        }
        request_waitable_root_graceful_termination(&self.live_root)
    }

    pub fn request_force_termination(&self) -> Result<bool> {
        if self.waited_status.is_some() {
            return Ok(false);
        }
        signal_exact_process(&self.live_root, ExactProcessSignal::ForceTermination)
    }
}
