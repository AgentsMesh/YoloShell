use super::*;
use crate::host::resume::{ResumeTarget, ResumeTransaction};
use crate::paths::{Paths, read_json};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;
use yolo_startup_checkpoint_contract::{
    StartupCheckpointEvidence, StartupLaunchFence, StartupPhase, StartupProcessIdentity,
    StartupStage, StartupStatus,
};

pub use super::resume::resume_terminal_rearm_test_hook::{
    Boundary, CRASH_BOUNDARY_ENVIRONMENT, configured_boundary,
};

include!("failed_rearm_process_fixture_constants.rs");
include!("failed_rearm_process_fixture_evidence.rs");
include!("failed_rearm_process_fixture_prepare.rs");
include!("failed_rearm_process_fixture_crash.rs");
include!("failed_rearm_process_fixture_crash_state.rs");
include!("failed_rearm_process_fixture_recover.rs");
include!("failed_rearm_process_fixture_recovery_verify.rs");
