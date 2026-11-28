//! Time-coalesced snapshots of the Host terminal catalogue.
//!
//! A Hub command reads the catalogue several times (listing, agent projection,
//! current-agent resolution). The Host builds one view per session, so serving
//! those reads from one snapshot removes O(sessions) work per repeated read.
//! Catalogue mutations invalidate the snapshot.

use std::collections::BTreeSet;
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use uuid::Uuid;
use yolo_terminal_model::TerminalView;

mod snapshot;
use snapshot::CachedCatalogue;

/// How long one snapshot serves reads: long enough to coalesce the reads of
/// one client command, short enough to see worker-driven changes.
pub const CATALOGUE_TTL: Duration = Duration::from_millis(2_000);

pub struct CatalogueCache {
    state: Mutex<State>,
    ttl: Duration,
}

#[derive(Default)]
struct State {
    entry: Option<CachedCatalogue>,
    watch: Weak<()>,
}

impl CatalogueCache {
    /// A zero TTL disables reuse; protocol tests answer one Host request per call.
    pub fn new(ttl: Duration) -> Self {
        Self {
            state: Mutex::default(),
            ttl,
        }
    }

    /// A complete snapshot also serves archived-free reads; the reverse never
    /// holds, so an archived-free read cannot hide archived sessions from a
    /// later complete read.
    pub async fn views(
        &self,
        client: &yolo_client::Client,
        include_archived: bool,
    ) -> Result<Vec<TerminalView>, yolo_client::ClientError> {
        if self.ttl.is_zero() {
            return client.list_sessions_complete(include_archived).await;
        }
        // The lock spans the Host read so concurrent readers share one call.
        let mut state = self.state.lock().await;
        let State { entry, watch } = &mut *state;
        if let Some(cached) = entry.as_mut().filter(|c| c.complete || !include_archived) {
            let expired = cached.fetched_at.elapsed() >= self.ttl;
            if cached.dirty.is_empty() && !expired {
                return Ok(cached.select(include_archived));
            }
            if watch.strong_count() > 0
                && cached.refresh_live(client, expired).await
                && watch.strong_count() > 0
            {
                return Ok(cached.select(include_archived));
            }
        }
        *entry = None;
        let views = client.list_sessions_complete(include_archived).await?;
        *entry = Some(CachedCatalogue {
            views: views.clone(),
            complete: include_archived,
            fetched_at: Instant::now(),
            dirty: Default::default(),
        });
        Ok(views)
    }

    /// The event reader owns this lease for the lifetime of its authenticated
    /// subscription. Cancellation and disconnect automatically restore the TTL.
    pub async fn follow_lifecycle(&self) -> Arc<()> {
        let lease = Arc::new(());
        let mut state = self.state.lock().await;
        state.watch = Arc::downgrade(&lease);
        state.entry = None;
        lease
    }

    pub async fn invalidate(&self) {
        self.state.lock().await.entry = None;
    }

    pub async fn changed(&self, ids: &BTreeSet<Uuid>) {
        let mut state = self.state.lock().await;
        if let Some(cached) = &mut state.entry {
            let known = cached
                .views
                .iter()
                .filter(|row| ids.contains(&row.manifest.terminal_id))
                .count();
            if known == ids.len() {
                cached.dirty.extend(ids);
            } else {
                state.entry = None;
            }
        }
    }
}
