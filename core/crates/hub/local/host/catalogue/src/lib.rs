//! Time-coalesced snapshots of the Host terminal catalogue.
//!
//! A Hub command reads the catalogue several times (listing, agent projection,
//! current-agent resolution). The Host builds one view per session, so serving
//! those reads from one snapshot removes O(sessions) work per repeated read.
//! Catalogue mutations invalidate the snapshot.

use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use yolo_terminal_model::TerminalView;

/// How long one snapshot serves reads: long enough to coalesce the reads of
/// one client command, short enough to see worker-driven changes.
pub const CATALOGUE_TTL: Duration = Duration::from_millis(2_000);

pub struct CatalogueCache {
    entry: Mutex<Option<CachedCatalogue>>,
    ttl: Duration,
}

struct CachedCatalogue {
    views: Vec<TerminalView>,
    complete: bool,
    fetched_at: Instant,
}

impl CatalogueCache {
    /// A zero TTL disables reuse; protocol tests answer one Host request per call.
    pub fn new(ttl: Duration) -> Self {
        Self {
            entry: Mutex::new(None),
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
        let mut entry = self.entry.lock().await;
        if let Some(cached) = entry.as_ref()
            && cached.fetched_at.elapsed() < self.ttl
            && (cached.complete || !include_archived)
        {
            if cached.complete && !include_archived {
                return Ok(cached
                    .views
                    .iter()
                    .filter(|view| !view.manifest.archived)
                    .cloned()
                    .collect());
            }
            return Ok(cached.views.clone());
        }
        let views = client.list_sessions_complete(include_archived).await?;
        *entry = Some(CachedCatalogue {
            views: views.clone(),
            complete: include_archived,
            fetched_at: Instant::now(),
        });
        Ok(views)
    }

    pub async fn invalidate(&self) {
        *self.entry.lock().await = None;
    }
}
