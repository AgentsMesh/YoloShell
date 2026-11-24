use super::MacosSecondaryCustody;
use crate::process::inventory_stable::StableInventory;
use crate::process::pty_session_proof::macos_descriptor_capability;
use crate::process::signal_authority::{ExactProcessSignal, signal_exact_process};
use crate::process::{LiveProcessIdentity, MacosProcessCapabilityRelease, ProcessIdentity};
use crate::pty_custody::CustodyReleaseSettlement;
use anyhow::Result;

const FALLBACK_OBSERVATION_DETAIL: &str =
    "confirmed-bootstrap fallback requires an exact premature capability release";

impl MacosSecondaryCustody {
    pub fn confirm_root_exit(&self) -> Result<()> {
        self.process_reader.reconcile_armed_root_exit()
    }

    pub fn closure_settled(&self) -> Result<bool> {
        self.process_reader
            .poll_release()
            .map(|release| release == MacosProcessCapabilityRelease::Released)
    }

    pub fn process_capability_observation(
        &self,
    ) -> Option<crate::process::MacosProcessCapabilityObservation> {
        self.process_reader.liveness_observation()
    }

    pub fn closure_settled_for(&self, settlement: &CustodyReleaseSettlement) -> Result<bool> {
        if !settlement.requires_confirmed_bootstrap_global_exclusion() {
            return self.closure_settled();
        }
        anyhow::ensure!(
            self.process_reader.liveness_observation()
                == Some(crate::process::MacosProcessCapabilityObservation::PrematurelyReleased),
            FALLBACK_OBSERVATION_DETAIL
        );
        Ok(true)
    }

    pub fn request_active_settlement(&self) -> Result<bool> {
        if self.closure_settled()? {
            let runtime = &self.identity.runtime;
            let child = ProcessIdentity {
                pid: runtime.child_pid,
                start_time: runtime.child_start_time,
            };
            let Some(live) = LiveProcessIdentity::capture(child)? else {
                return Ok(true);
            };
            signal_exact_process(&live, ExactProcessSignal::ForceTermination)?;
            return Ok(false);
        }
        let inventory = StableInventory::capture()?;
        for entry in inventory.entries().iter().copied() {
            let live = entry.live_identity()?;
            if !live.try_matches()? {
                continue;
            }
            let holds_writer = macos_descriptor_capability::exact_descriptor_holder(
                entry.pid,
                self.process_reader.writer_handle(),
            );
            match holds_writer {
                Ok(true) if live.try_matches()? => {
                    signal_exact_process(&live, ExactProcessSignal::ForceTermination)?;
                }
                Ok(_) | Err(_) => {}
            }
        }
        Ok(false)
    }
}
