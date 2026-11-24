pub(crate) const MONITOR_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);
pub(crate) const IDENTITY_MISMATCH: &str =
    "public client access entry Hub does not match its dispatcher";
pub(crate) const STORAGE_POISONED: &str = "public client access store owner is poisoned";
pub(crate) const AUTHORITY_CHANGED: &str =
    "persisted public client access differs from live authority";
