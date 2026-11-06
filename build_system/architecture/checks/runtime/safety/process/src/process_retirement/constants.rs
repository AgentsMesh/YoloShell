pub const OWNER_PATH: &str = "core/crates/runtime/engine/process/src/captured/captured_tree.rs";
pub const CALLER_PATH: &str =
    "core/crates/runtime/engine/process/src/captured/tree/gone/captured_tree_gone_retirement.rs";
pub const BOOTSTRAP_CALLER_PATH: &str =
    "core/crates/runtime/engine/process/src/captured/tree/captured_tree_bootstrap_failure.rs";
pub const TREE_OWNER: &str = "CapturedProcessTree";
pub const RETIRE_METHOD: &str = "retire_session";
pub const DIRECT_OWNER: &str = "DirectRetirementReady";
pub const DIRECT_COMMIT: &str = "commit";
pub const MASTER_OWNER: &str = "MasterReleasePendingProducersGone";
pub const MASTER_COMMIT: &str = "commit_retirement_after_master_release";
pub const BOOTSTRAP_OWNER: &str = "ExplicitBootstrapFailureRetirementReady";
pub const BOOTSTRAP_COMMIT: &str = "commit";
pub const EXPECTED_DEFINITIONS: usize = 1;
pub const EXPECTED_DIRECT_CALLS: usize = 1;
pub const EXPECTED_MASTER_CALLS: usize = 1;
pub const EXPECTED_BOOTSTRAP_CALLS: usize = 1;
pub const OWNERSHIP_MESSAGE: &str =
    "session retirement is reserved for the typed gone-retirement commit boundary";
pub const DEFINITION_MESSAGE: &str =
    "CapturedProcessTree retirement must remain private and unique";
pub const CALLER_MESSAGE: &str = "typed retirement commits must each retire exactly once";
