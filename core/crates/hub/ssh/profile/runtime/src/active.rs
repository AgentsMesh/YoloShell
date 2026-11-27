use crate::{SshProfileRuntime, constants::*};
use anyhow::{Result, ensure};
use std::sync::{Arc, Mutex, Weak};
use yolo_broker_tunnel::BrokerTunnelClose;
use yolo_hub_ssh_profiles::{SshBrokerProfile, SshProfileConfiguration};

pub(crate) struct ActiveTunnel {
    registration: SshBrokerProfile,
    close: BrokerTunnelClose,
    live: Weak<()>,
}

impl SshProfileRuntime {
    /// Caller holds the profile store lock through verification and registration.
    pub(crate) fn retain_tunnel(
        &self,
        registration: SshBrokerProfile,
        close: BrokerTunnelClose,
    ) -> Result<Arc<()>> {
        let mut active = self.active.lock().map_err(|_| anyhow::anyhow!(POISONED))?;
        active.retain(|entry| entry.live.strong_count() != 0);
        ensure!(
            active.len() < yolo_hub_federation_settings::MAX_PEERS,
            TUNNEL_CAPACITY
        );
        let live = Arc::new(());
        active.push(ActiveTunnel {
            registration,
            close,
            live: Arc::downgrade(&live),
        });
        Ok(live)
    }
    /// Close revoked transports before acknowledging the registry change.
    pub(crate) fn revoke_changed(&self, configuration: &SshProfileConfiguration) -> Result<()> {
        let mut active = self.active.lock().map_err(|_| anyhow::anyhow!(POISONED))?;
        active.retain(|entry| {
            if entry.live.strong_count() == 0 {
                return false;
            }
            if configuration.profiles.contains(&entry.registration) {
                return true;
            }
            entry.close.close();
            false
        });
        Ok(())
    }
    pub(crate) fn close_all(&self) {
        close_all(&self.active);
    }
}

pub(crate) fn close_all(active: &Mutex<Vec<ActiveTunnel>>) {
    let mut active = active.lock().unwrap_or_else(|error| error.into_inner());
    for entry in active.drain(..) {
        entry.close.close();
    }
}
