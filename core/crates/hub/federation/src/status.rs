use crate::node::{Federation, now_ms};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, MutexGuard},
};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::ConsumerSummary;

impl Federation {
    /// Currently open directed links, for diagnostics and acceptance checks.
    pub async fn connected_peers(&self) -> Vec<HubId> {
        self.connections
            .read()
            .await
            .iter()
            .filter(|(_, link)| !link.is_closed())
            .map(|(hub, _)| *hub)
            .collect()
    }
}

struct Session {
    since_ms: u64,
    links: u16,
}

/// Live inbound links keyed by the importing machine's Hub identity.
#[derive(Default)]
pub(crate) struct ConsumerRegistry {
    sessions: Mutex<BTreeMap<HubId, Session>>,
}

impl ConsumerRegistry {
    fn lock(&self) -> MutexGuard<'_, BTreeMap<HubId, Session>> {
        self.sessions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Removes one inbound link when the peer session ends.
pub(crate) struct ConsumerGuard {
    federation: Arc<Federation>,
    hub: HubId,
}

impl Federation {
    pub(crate) fn enter_consumer(self: &Arc<Self>, peer: PeerIdentity) -> ConsumerGuard {
        let hub = peer.hub();
        let mut sessions = self.consumers.lock();
        let session = sessions.entry(hub).or_insert_with(|| Session {
            since_ms: now_ms().unwrap_or_default(),
            links: 0,
        });
        session.links = session.links.saturating_add(1);
        ConsumerGuard {
            federation: self.clone(),
            hub,
        }
    }

    pub fn consumers(&self) -> Vec<ConsumerSummary> {
        self.consumers
            .lock()
            .iter()
            .map(|(hub, session)| ConsumerSummary {
                hub: *hub,
                connected_since_ms: session.since_ms,
                links: session.links,
            })
            .collect()
    }
}

impl Drop for ConsumerGuard {
    fn drop(&mut self) {
        let mut sessions = self.federation.consumers.lock();
        let Some(session) = sessions.get_mut(&self.hub) else {
            return;
        };
        session.links = session.links.saturating_sub(1);
        if session.links == 0 {
            sessions.remove(&self.hub);
        }
    }
}
