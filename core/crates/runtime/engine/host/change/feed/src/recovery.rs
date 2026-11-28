use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Only sessions with outstanding supervision or recovery work occupy the queue.
#[derive(Default)]
pub struct RecoveryQueue {
    pending: Mutex<HashMap<Uuid, Arc<()>>>,
    discovered: AtomicBool,
}

#[derive(Clone)]
pub struct RecoveryTicket {
    pub terminal_id: Uuid,
    generation: Arc<()>,
}

impl RecoveryQueue {
    pub fn needs_discovery(&self) -> bool {
        !self.discovered.load(Ordering::Acquire)
    }

    fn finish_discovery(&self) {
        self.discovered.store(true, Ordering::Release);
    }

    pub fn schedule(&self, terminal_id: Uuid) -> RecoveryTicket {
        let generation = Arc::new(());
        self.pending
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(terminal_id, generation.clone());
        RecoveryTicket {
            terminal_id,
            generation,
        }
    }

    pub fn pending(&self) -> Vec<RecoveryTicket> {
        self.pending
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .iter()
            .map(|(&terminal_id, generation)| RecoveryTicket {
                terminal_id,
                generation: generation.clone(),
            })
            .collect()
    }

    /// Match the catalogue read to its input tickets. Missing and archived
    /// rows retire their old ticket. A repair failure issues a fresh ticket.
    pub fn selected(
        &self,
        batch: Option<Vec<RecoveryTicket>>,
        ids: impl Iterator<Item = Uuid>,
    ) -> Vec<RecoveryTicket> {
        let Some(batch) = batch else {
            let selected = ids.map(|id| self.schedule(id)).collect();
            self.finish_discovery();
            return selected;
        };
        let mut remaining: HashMap<_, _> = batch.into_iter().map(|t| (t.terminal_id, t)).collect();
        let selected = ids.filter_map(|id| remaining.remove(&id)).collect();
        for ticket in remaining.values() {
            self.settle(ticket);
        }
        selected
    }

    /// A later publication owns a distinct ticket, even for the same terminal.
    pub fn settle(&self, ticket: &RecoveryTicket) -> bool {
        let mut pending = self.pending.lock().unwrap_or_else(|p| p.into_inner());
        if pending
            .get(&ticket.terminal_id)
            .is_some_and(|current| Arc::ptr_eq(current, &ticket.generation))
        {
            pending.remove(&ticket.terminal_id);
            return true;
        }
        false
    }
}
