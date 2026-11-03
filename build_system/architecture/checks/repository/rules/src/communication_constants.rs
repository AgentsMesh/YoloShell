pub(super) const CORE: &str = "core/";
pub(super) const SNAPSHOT_STORAGE: &str = "core/crates/persistence/journal/";
pub(super) const RETIRED_VALUE_CRATE: &str = "rmpv";
pub(super) const SNAPSHOT_CODECS: &[&str] = &["rmp_serde", "rmp-serde"];
pub(super) const RETIRED_VALUE_MESSAGE: &str =
    "rmpv is retired; internal domain values use typed Cap'n Proto structural values";
pub(super) const CODEC_SCOPE_MESSAGE: &str = "MessagePack is restricted to the persisted journal snapshot format, never process or Hub protocols";
