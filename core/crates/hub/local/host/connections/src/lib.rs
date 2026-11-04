use std::{collections::BTreeMap, sync::Arc, time::Instant};
use tokio::sync::Mutex;
use uuid::Uuid;

const DEFAULT_CAPACITY: usize = 64;
pub const INPUT_CONNECTION_IDLE_BUDGET: std::time::Duration =
    std::time::Duration::from_secs(4);

pub struct HostConnectionState {
    pub connection: Option<yolo_client::RpcConnection>,
    pub last_used: Instant,
}

impl HostConnectionState {
    pub fn idle(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.last_used) >= INPUT_CONNECTION_IDLE_BUDGET
    }
}

pub type HostConnection = Arc<Mutex<HostConnectionState>>;

/// Bounded connection slots keyed by the runtime session they serve.
///
/// A slot is shared only by requests for one runtime. Idle slots are evicted
/// before a new one is admitted, so a long-lived Hub cannot retain one entry
/// for every historical terminal.
#[derive(Clone)]
pub struct HostConnections {
    slots: Arc<Mutex<BTreeMap<Uuid, HostConnection>>>,
    capacity: usize,
}

impl Default for HostConnections {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }
}

impl HostConnections {
    #[cfg(test)]
    pub fn with_capacity(capacity: usize) -> Self {
        assert!(capacity > 0);
        Self {
            slots: Arc::new(Mutex::new(BTreeMap::new())),
            capacity,
        }
    }

    #[cfg(not(test))]
    fn with_capacity(capacity: usize) -> Self {
        Self {
            slots: Arc::new(Mutex::new(BTreeMap::new())),
            capacity,
        }
    }

    pub async fn slot(&self, session: Uuid) -> HostConnection {
        let mut slots = self.slots.lock().await;
        if let Some(slot) = slots.get(&session) {
            return slot.clone();
        }
        if slots.len() >= self.capacity {
            let evict = slots.iter().find_map(|(id, slot)| {
                (Arc::strong_count(slot) == 1 && slot.try_lock().is_ok()).then_some(*id)
            });
            if let Some(id) = evict {
                slots.remove(&id);
            } else {
                // Busy slots cannot be evicted safely. The request still
                // proceeds on an uncached connection; the retained map stays
                // bounded and the next request can retry reuse.
                return Arc::new(Mutex::new(HostConnectionState {
                    connection: None,
                    last_used: Instant::now(),
                }));
            }
        }
        let slot = Arc::new(Mutex::new(HostConnectionState {
            connection: None,
            last_used: Instant::now(),
        }));
        slots.insert(session, slot.clone());
        slot
    }

    #[cfg(test)]
    pub async fn len(&self) -> usize {
        self.slots.lock().await.len()
    }
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
