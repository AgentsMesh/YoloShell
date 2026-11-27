#[doc(hidden)]
pub mod constants;
#[path = "fence.rs"]
mod fence;
#[path = "reboot_validation.rs"]
mod reboot_validation;
#[path = "schema.rs"]
mod schema;
#[doc(hidden)]
pub mod validation;

pub use fence::{from_evidence, matches_runtime, validate as validate_launch_fence};
pub use reboot_validation::source_before_reboot_migration;
pub use schema::{
    StartupCheckpointEvidence, StartupLaunchFence, StartupPhase, StartupProcessIdentity,
    StartupPtyCustodyRollover, StartupStage, StartupStatus,
};
