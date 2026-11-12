pub(crate) const UNKNOWN_PEER: &str = "peer is not pinned for this directory";
pub(crate) const STALE_LINK: &str = "directory callback belongs to a replaced peer connection";
pub(crate) const STALE_REVISION: &str = "peer directory revision did not advance";
pub(crate) const UNKNOWN_ORIGIN: &str = "directory origin is not explicitly pinned";
pub(crate) const DUPLICATE_PATH: &str = "directory repeats a delegated route";
pub(crate) const CAPACITY: &str = "directory exceeds configured capacity";
pub(crate) const CONFLICTING_ORIGIN: &str = "origin reused a revision for different catalogue data";
pub(crate) const INVALID_PINS: &str = "origin pin conflicts with an existing identity";
pub(crate) const HISTORY_ENTRY_MISMATCH: &str =
    "observation history belongs to another entry identity";
pub(crate) const HISTORY_TARGET_BUSY: &str =
    "observation history requires a generation without active links or route proofs";
pub(crate) const HISTORY_DUPLICATE_ORIGIN: &str = "observation history repeats an origin";
pub(crate) const HISTORY_LOCAL_ORIGIN: &str = "observation history cannot claim the entry origin";
pub(crate) const HISTORY_IDENTITY_CONFLICT: &str = "observation history origin identity conflicts";
