use futures::{StreamExt, stream};
use std::{collections::BTreeSet, time::Instant};
use uuid::Uuid;
use yolo_terminal_model::{TerminalRuntimeStatus, TerminalView};

const PARALLEL_READS: usize = 8;

pub(super) struct CachedCatalogue {
    pub(super) views: Vec<TerminalView>,
    pub(super) complete: bool,
    pub(super) fetched_at: Instant,
    pub(super) dirty: BTreeSet<Uuid>,
}

impl CachedCatalogue {
    pub(super) fn select(&self, include_archived: bool) -> Vec<TerminalView> {
        self.views
            .iter()
            .filter(|row| include_archived || !row.manifest.archived)
            .cloned()
            .collect()
    }

    /// A connected lifecycle subscription invalidates membership and settled rows.
    /// Active rows still need current runtime status and output positions.
    pub(super) async fn refresh_live(
        &mut self,
        client: &yolo_client::Client,
        expired: bool,
    ) -> bool {
        let live: Vec<_> = self
            .views
            .iter()
            .enumerate()
            .filter(|(_, row)| {
                self.dirty.contains(&row.manifest.terminal_id) || expired && !settled(row)
            })
            .map(|(index, row)| (index, row.manifest.terminal_id))
            .collect();
        if live.is_empty() {
            return true;
        }
        // A bulk read is cheaper when most rows need observation. Its Host-side
        // concurrency also bounds the time spent on large active inventories.
        if live.len() > self.views.len() / 2 {
            return false;
        }
        let mut reads = stream::iter(live)
            .map(|(index, id)| async move { (index, id, client.get_session(id).await) })
            .buffered(PARALLEL_READS);
        let mut fresh = Vec::new();
        while let Some((index, id, result)) = reads.next().await {
            let Ok(row) = result else { return false };
            if row.manifest.terminal_id != id
                || self.views[index].manifest.archived != row.manifest.archived
            {
                return false;
            }
            fresh.push((index, row));
        }
        for (index, row) in fresh {
            self.views[index] = row;
        }
        self.dirty.clear();
        if expired {
            self.fetched_at = Instant::now();
        }
        true
    }
}

fn settled(row: &TerminalView) -> bool {
    row.manifest.archived
        || row.runtime.as_ref().is_some_and(|runtime| {
            matches!(
                runtime.status,
                TerminalRuntimeStatus::Exited | TerminalRuntimeStatus::Lost
            )
        })
}
