use tokio::sync::watch;
use yolo_hub_client_access_protocol::ClientGrant;
use yolo_hub_identity::HubId;

/// A snapshot of public rights bound to one registry lifetime and revision.
/// Possession never grants federation or local-management authority.
#[derive(Clone, Debug)]
pub struct ClientAdmission {
    grant: ClientGrant,
    entry_hub: HubId,
    revision: u64,
    changes: watch::Receiver<Option<u64>>,
}

impl ClientAdmission {
    pub(crate) fn new(
        grant: ClientGrant,
        entry_hub: HubId,
        revision: u64,
        changes: watch::Receiver<Option<u64>>,
    ) -> Self {
        Self {
            grant,
            entry_hub,
            revision,
            changes,
        }
    }

    pub fn grant(&self) -> &ClientGrant {
        &self.grant
    }

    pub fn entry_hub(&self) -> HubId {
        self.entry_hub
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn is_current(&self, now_ms: u64) -> bool {
        now_ms < self.grant.expires_at_ms
            && self.changes.has_changed().is_ok()
            && *self.changes.borrow() == Some(self.revision)
    }

    /// Waits for policy replacement, close, or loss of every registry owner.
    /// The caller separately enforces grant expiry with its operation deadline.
    pub async fn revoked(&self) {
        let mut changes = self.changes.clone();
        loop {
            if *changes.borrow_and_update() != Some(self.revision) {
                return;
            }
            if changes.changed().await.is_err() {
                return;
            }
        }
    }
}
