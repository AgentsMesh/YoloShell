use super::*;

#[path = "checkpoint/startup_checkpoint.rs"]
pub(crate) mod startup_checkpoint;
pub(crate) mod startup_cleanup_owner;
pub(crate) mod startup_cleanup_scope;
pub(crate) mod startup_custody_transfer;
#[path = "guard/startup_guard.rs"]
pub(crate) mod startup_guard;
pub(crate) mod startup_resources;
pub(crate) mod startup_scope;
#[path = "untracked/startup_untracked_proof.rs"]
pub(crate) mod startup_untracked_proof;
#[path = "untracked/startup_untracked_reaper.rs"]
pub(crate) mod startup_untracked_reaper;

include!("startup.rs");
