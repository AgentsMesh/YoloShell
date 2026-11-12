pub const RETIRED_OPERATIONS: &[&str] = &["receipt", "generate"];
pub const AUDIT_TOOL_ARGUMENT: &str = "audit-tool";
pub const RETIRED_ARGUMENTS: &[&str] = &[
    yolo_release_archive_audit::constants::ARCHIVE_ARGUMENT,
    AUDIT_TOOL_ARGUMENT,
    "bundle-policy",
    "gate-receipt",
    "gate",
];
pub const FAKE_AUDIT_TOOL: &str = "fake-audit-tool";
pub const MISSING_FILE: &str = "missing-artifact";
pub const EXISTING_OUTPUT: &str = "existing-output";
pub const MISSING_PARENT: &str = "missing-parent";
pub const DELIVERY_NAME: &str = "delivery";
pub const INVALID_NOTARY_PROFILE: &str = "invalid profile";
pub const CURRENT_DIRECTORY: &str = ".";
