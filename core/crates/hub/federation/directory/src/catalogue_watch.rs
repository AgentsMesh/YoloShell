use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use uuid::Uuid;
use yolo_hub_identity::HubId;
use yolo_hub_peer_protocol::Catalogue;
use yolo_hub_peer_protocol::Signed;

#[derive(Clone)]
pub struct CatalogueChanges {
    pub(crate) epoch: Uuid,
    pub(crate) versions: watch::Sender<Option<RevisionState>>,
}

pub(crate) struct RevisionState {
    pub(crate) origins: BTreeMap<HubId, Versions>,
    visible: Uuid,
}

#[derive(Clone, Copy)]
pub(crate) struct Versions {
    visible: Uuid,
    pub(crate) advertisement: Uuid,
}

impl Default for CatalogueChanges {
    fn default() -> Self {
        let epoch = Uuid::new_v4();
        let state = RevisionState {
            origins: BTreeMap::new(),
            visible: epoch,
        };
        Self {
            epoch,
            versions: watch::channel(Some(state)).0,
        }
    }
}

impl CatalogueChanges {
    /// Only the directory owner inserts identities, after accepting new evidence.
    pub fn touch(&self, origins: impl IntoIterator<Item = HubId>) {
        self.revise(origins, true)
    }

    pub(crate) fn revise(&self, origins: impl IntoIterator<Item = HubId>, visible: bool) {
        self.versions.send_if_modified(|state| {
            let Some(versions) = state else { return false };
            let mut modified = false;
            for origin in origins {
                let version = versions.origins.entry(origin).or_insert(Versions {
                    visible: self.epoch,
                    advertisement: self.epoch,
                });
                version.advertisement = Uuid::new_v4();
                if visible {
                    version.visible = version.advertisement;
                }
                modified = true;
            }
            if modified && visible {
                versions.visible = Uuid::new_v4();
            }
            modified
        });
    }

    pub fn publish(
        &self,
        latest: &watch::Sender<Option<Arc<Signed<Catalogue>>>>,
        value: Option<&Signed<Catalogue>>,
    ) {
        let current = value.cloned().map(Arc::new);
        let previous = latest.send_replace(current.clone());
        if let Some(new) = current {
            self.revise(
                [new.signer.hub()],
                inventory_changed(previous.as_ref().map(|v| &v.payload), &new.payload),
            );
        } else if let Some(old) = previous {
            self.touch([old.signer.hub()]);
        }
    }

    pub fn retire(&self) {
        self.versions.send_replace(None);
    }

    /// Subscribe before inspecting the cursor so a change during the caller's
    /// snapshot is immediately observable on its next wait.
    pub async fn wait(
        &self,
        after: Option<Uuid>,
        wait_ms: u32,
    ) -> Option<yolo_hub_rpc_protocol::Reply> {
        let mut changes = self.versions.subscribe();
        if wait_ms != 0 && after.is_some() {
            let changed = changes.wait_for(|state| {
                state
                    .as_ref()
                    .is_none_or(|state| Some(state.visible) != after)
            });
            let _ = tokio::time::timeout(Duration::from_millis(u64::from(wait_ms)), changed).await;
        }
        let current = changes.borrow();
        let state = current.as_ref()?;
        let origins = (after != Some(state.visible)).then(|| {
            state
                .origins
                .iter()
                .map(|(&hub, revision)| (hub, revision.visible))
                .collect()
        });
        Some(yolo_hub_rpc_protocol::Reply::CatalogueCursors {
            epoch: self.epoch,
            cursor: state.visible,
            origins,
        })
    }
}

/// Publication times and signed revisions also advance on a heartbeat.
pub fn inventory_changed(previous: Option<&Catalogue>, new: &Catalogue) -> bool {
    previous.is_none_or(|old| {
        old.policy_epoch != new.policy_epoch
            || old.terminals != new.terminals
            || old.agents != new.agents
            || old.runs != new.runs
            || old.workspaces != new.workspaces
    })
}
