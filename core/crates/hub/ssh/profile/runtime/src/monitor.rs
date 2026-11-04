use crate::constants::*;
use std::sync::{Mutex, Weak};
use tokio::{sync::watch, task::JoinHandle, time::interval};
use yolo_broker_tunnel::BrokerTunnelClose;
use yolo_hub_ssh_profiles::{SshBrokerProfile, SshProfileStore};
use yolo_hub_state::HubState;

pub(crate) fn watch(
    state: Weak<HubState>,
    store: Weak<Mutex<SshProfileStore>>,
    active: Weak<Mutex<Vec<crate::active::ActiveTunnel>>>,
    registration: SshBrokerProfile,
    mut changed: watch::Receiver<u64>,
    close: BrokerTunnelClose,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut check = interval(PROFILE_RECHECK);
        loop {
            tokio::select! {
                result = changed.changed() => if result.is_err() { break; },
                _ = check.tick() => {},
            }
            let (Some(state), Some(store)) = (state.upgrade(), store.upgrade()) else {
                break;
            };
            let current = state.verify().and_then(|()| {
                store
                    .lock()
                    .map_err(|_| anyhow::anyhow!(POISONED))?
                    .current()
            });
            match current {
                Ok(current) if current.configuration.profiles.contains(&registration) => {}
                Ok(_) => break,
                Err(_) => {
                    if let Some(active) = active.upgrade() {
                        crate::active::close_all(&active);
                    }
                    break;
                }
            }
        }
        close.close();
    })
}
