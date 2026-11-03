pub use yolo_architecture_foundation::MIN_SEMANTIC_LITERAL_LENGTH;
pub use yolo_architecture_foundation::text::joined;

pub const PROTOCOL_CONTEXT_LOOKBACK: usize = 2;
pub const CLI_PREFIX: &str = "--";
pub const SERDE_ATTRIBUTE_OPEN: &str = "#[serde(";
pub const SERDE_ATTRIBUTE_CLOSE: &str = ")]";
pub const SERDE_RENAME_ALL_KEY: &str = "rename_all";
pub const SERDE_TAG_KEY: &str = "tag";
pub const SERDE_WITH_KEY: &str = "with";
pub const SERDE_BYTES_ADAPTER: &str = "serde_bytes";
pub const RUST_UI_TERMS: &[&[&str]] = &[
    &["app", "kit"],
    &["swift", "term"],
    &["co", "coa"],
    &["ob", "jc"],
    &["wi", "nit"],
    &["ic", "ed"],
    &["sl", "int"],
    &["tau", "ri"],
];
pub const REQUIRED_CENTRAL_LITERALS: &[&str] = &[
    "acquire_writer",
    "renew_writer",
    "release_writer",
    "takeover_writer",
    "writer_control_changed",
    "writer_lease_acquired",
    "writer_lease_revoked",
    "writer_lease_expired",
    "writer_lease_conflicted",
    "controlled_elsewhere",
    "reconnecting",
    "reacquiring",
    "held_by_other",
    "lease_expired",
    "not_holder",
    "stale_generation",
    "transport_unavailable",
    "session_not_running",
];
pub const OPERATIONAL_MODE_MARKER: &str = "mode";
