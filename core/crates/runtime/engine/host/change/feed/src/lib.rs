//! Host change feed: which requests change durable Host state, and one
//! revision-stamped notification stream for catalogue subscribers.
//!
//! Notifications are hints, not the catalogue. A subscriber whose previous
//! revision differs (or that lagged) performs one full read and resumes.
//! Worker binding changes arrive on the Worker's agent-event stream, which
//! never carries terminal output.

use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::broadcast;
use uuid::Uuid;
use yolo_capnp_rpc::{EventSource, Future, Value};
use yolo_runtime_protocol::CatalogueChange;
use yolo_wire::{EventEnvelope, event, method};

const CHANGE_BUFFER: usize = 64;

/// Requests that can change state a catalogue subscriber observes.
pub fn mutates(name: &str) -> bool {
    matches!(
        name,
        method::CREATE_SESSION
            | method::ACQUIRE_WRITER
            | method::RENEW_WRITER
            | method::RELEASE_WRITER
            | method::TAKEOVER_WRITER
            | method::TERMINAL_INPUT
            | method::RESIZE_TERMINAL
            | method::SIGNAL_SESSION
            | method::INTERRUPT_NATIVE_AGENT
            | method::SUBMIT_NATIVE_AGENT
            | method::RESPOND_NATIVE_AGENT
            | method::TERMINATE_SESSION
            | method::ARCHIVE_SESSION
            | method::DISCARD_SESSION
            | method::CLEANUP_TERMINATE_SESSION
            | method::CLEANUP_ARCHIVE_SESSION
            | method::CLEANUP_DISCARD_SESSION
            | method::FINALIZE_CREATION
            | method::RESUME_SESSION_EXACT
            | method::RECONCILE
    )
}

/// Requests that can change what a catalogue subscriber observes. Writer
/// lease traffic and terminal input do not: only lifecycle and creation
/// transitions, control effects and reconciliation reach the catalogue.
pub fn changes_catalogue(name: &str) -> bool {
    matches!(
        name,
        method::CREATE_SESSION
            | method::FINALIZE_CREATION
            | method::INTERRUPT_NATIVE_AGENT
            | method::SUBMIT_NATIVE_AGENT
            | method::RESPOND_NATIVE_AGENT
            | method::TERMINATE_SESSION
            | method::ARCHIVE_SESSION
            | method::DISCARD_SESSION
            | method::CLEANUP_TERMINATE_SESSION
            | method::CLEANUP_ARCHIVE_SESSION
            | method::CLEANUP_DISCARD_SESSION
            | method::RESUME_SESSION_EXACT
            | method::RECONCILE
    )
}

pub struct ChangeFeed {
    revision: AtomicU64,
    changes: broadcast::Sender<CatalogueChange>,
}

impl Default for ChangeFeed {
    fn default() -> Self {
        Self::new()
    }
}

impl ChangeFeed {
    pub fn new() -> Self {
        let (changes, _) = broadcast::channel(CHANGE_BUFFER);
        Self {
            revision: AtomicU64::new(0),
            changes,
        }
    }

    /// Record a change. Without a terminal id the whole catalogue may differ.
    pub fn touch(&self, terminal_id: Option<Uuid>) {
        let revision = self.revision.fetch_add(1, Ordering::SeqCst) + 1;
        let _ = self.changes.send(CatalogueChange {
            revision,
            terminal_id,
        });
    }

    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::SeqCst)
    }

    /// The current revision and the stream of changes after it. Subscribe
    /// first: a change racing the revision read is then either streamed or
    /// visible as a revision difference, never silently lost.
    pub fn subscribe(&self) -> (u64, ChangeSource) {
        let source = ChangeSource {
            changes: self.changes.subscribe(),
        };
        (self.revision(), source)
    }
}

pub struct ChangeSource {
    changes: broadcast::Receiver<CatalogueChange>,
}

impl EventSource for ChangeSource {
    fn next(&mut self) -> Future<'_, Value> {
        Box::pin(async move {
            let change = self.changes.recv().await.map_err(|error| match error {
                broadcast::error::RecvError::Closed => yolo_capnp_rpc::Error::Closed,
                broadcast::error::RecvError::Lagged(_) => yolo_capnp_rpc::Error::Closed,
            })?;
            let terminal_id = change.terminal_id.map(|id| id.to_string());
            let envelope = EventEnvelope::new(event::CATALOGUE_CHANGED, terminal_id, None, &change)
                .map_err(|error| yolo_capnp_rpc::Error::Remote(error.to_string()))?;
            yolo_capnp_codec::to_value(&envelope).map_err(Into::into)
        })
    }
}
