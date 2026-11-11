use super::constants::{ROUTE_PUBLICATION_CAPACITY, ROUTE_PUBLICATION_REVISION_INCREMENT};
use super::state::{RouteState, RouterEntries};
use super::{CodexRouteEntry, CodexRouteKey, CodexRouterError, CodexSessionRouter};
use std::sync::Arc;
use std::sync::atomic::Ordering;

#[derive(Clone)]
pub(in crate::worker::codex_hook) struct CodexCommittedRoute {
    pub(in crate::worker::codex_hook) revision: u64,
    pub(in crate::worker::codex_hook) entry: Arc<CodexRouteEntry>,
}

pub(in crate::worker::codex_hook) struct CodexRouteSubscription {
    pub(in crate::worker::codex_hook) snapshot: Vec<CodexCommittedRoute>,
    pub(in crate::worker::codex_hook) receiver:
        tokio::sync::broadcast::Receiver<CodexCommittedRoute>,
}

impl Default for RouterEntries {
    fn default() -> Self {
        Self::with_publication_clock(Arc::default())
    }
}

impl RouterEntries {
    fn with_publication_clock(publication_revision: Arc<std::sync::atomic::AtomicU64>) -> Self {
        let (publication_tx, _) = tokio::sync::broadcast::channel(ROUTE_PUBLICATION_CAPACITY);
        Self {
            entries: Default::default(),
            publication_revision,
            publication_tx,
            #[cfg(test)]
            faults: Default::default(),
        }
    }
}

impl RouterEntries {
    pub(super) fn settle_success(
        &self,
        key: CodexRouteKey,
        expected: &Arc<CodexRouteEntry>,
    ) -> Result<(), CodexRouterError> {
        #[cfg(test)]
        if let Some(error) = self.faults.settle.lock().unwrap().take() {
            return Err(error);
        }
        let mut entries = self.lock()?;
        let revision = expected.admission().record().route_revision();
        let duplicate = entries.values().filter_map(RouteState::active).any(|slot| {
            !Arc::ptr_eq(slot.entry(), expected) && slot.publication_revision() == Some(revision)
        });
        if duplicate {
            return Err(CodexRouterError::Conflict);
        }
        let slot = entries
            .get_mut(&key)
            .and_then(RouteState::active_mut)
            .filter(|slot| Arc::ptr_eq(slot.entry(), expected))
            .ok_or(CodexRouterError::Conflict)?;
        let publication = if slot.is_committed() {
            None
        } else {
            self.observe_publication_revision(revision)?;
            slot.commit(revision);
            Some(CodexCommittedRoute {
                revision,
                entry: Arc::clone(expected),
            })
        };
        let remove = slot.release();
        if remove {
            entries.remove(&key);
        }
        drop(entries);
        if let Some(publication) = publication {
            let _ = self.publication_tx.send(publication);
        }
        Ok(())
    }

    fn next_publication_revision(&self) -> Result<u64, CodexRouterError> {
        self.publication_revision
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                current.checked_add(ROUTE_PUBLICATION_REVISION_INCREMENT)
            })
            .map(|previous| previous + ROUTE_PUBLICATION_REVISION_INCREMENT)
            .map_err(|_| CodexRouterError::Capacity)
    }

    fn observe_publication_revision(&self, revision: u64) -> Result<(), CodexRouterError> {
        if revision == u64::default() {
            return Err(CodexRouterError::Admission);
        }
        self.publication_revision
            .fetch_max(revision, Ordering::AcqRel);
        Ok(())
    }

    fn committed_routes(&self) -> Result<Vec<CodexCommittedRoute>, CodexRouterError> {
        let entries = self.lock()?;
        let mut routes = entries
            .values()
            .filter_map(RouteState::active)
            .filter_map(|slot| {
                Some(CodexCommittedRoute {
                    revision: slot.publication_revision()?,
                    entry: Arc::clone(slot.entry()),
                })
            })
            .collect::<Vec<_>>();
        routes.sort_by_key(|route| route.revision);
        Ok(routes)
    }
}

impl CodexSessionRouter {
    pub(in crate::worker::codex_hook) fn with_publication_clock(
        paths: crate::paths::Paths,
        terminal_id: uuid::Uuid,
        runtime_epoch: uuid::Uuid,
        publication_revision: Arc<std::sync::atomic::AtomicU64>,
    ) -> Result<Self, CodexRouterError> {
        let mut router = Self::new(paths, terminal_id, runtime_epoch)?;
        router.inner = Arc::new(RouterEntries::with_publication_clock(publication_revision));
        Ok(router)
    }

    pub(in crate::worker::codex_hook) fn subscribe_committed(
        &self,
    ) -> Result<CodexRouteSubscription, CodexRouterError> {
        let receiver = self.inner.publication_tx.subscribe();
        let snapshot = self.inner.committed_routes()?;
        for route in &snapshot {
            route.entry.admission().verify()?;
        }
        Ok(CodexRouteSubscription { snapshot, receiver })
    }

    pub(in crate::worker::codex_hook) fn reserve_publication_revision(
        &self,
    ) -> Result<u64, CodexRouterError> {
        self.inner.next_publication_revision()
    }

    pub(in crate::worker::codex_hook) fn observe_persisted_admission(
        &self,
        admission: &crate::codex_hooks::CodexAdmissionAuthority,
    ) -> Result<(), CodexRouterError> {
        admission
            .verify()
            .map_err(|_| CodexRouterError::Admission)?;
        self.inner
            .observe_publication_revision(admission.record().route_revision())
    }
}
