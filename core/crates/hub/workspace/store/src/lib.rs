//! Durable workspace metadata with revision CAS and case-folded name ownership.
//! The store never creates or removes directories; the Host validates roots and
//! owns optional git worktree effects.

mod create;
mod error;
mod membership;
mod record;
mod reservation;
mod store;
mod worktree;

use uuid::Uuid;
use yolo_hub_terminal_protocol::PublicError;
use yolo_hub_workspace_protocol::WorkspaceAddress;

pub use error::StoreError;
pub(crate) use record::StoredWorkspace;
pub use reservation::WorkspaceReservation;
pub use store::WorkspaceStore;

#[cfg(test)]
use yolo_hub_identity::*;
#[cfg(test)]
use yolo_hub_workspace_protocol::*;

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
