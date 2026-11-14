pub(super) const QUARANTINE_PREFIX: &str = ".unbound-";
pub(super) const UNPROVEN: &str =
    "unbound socket namespace cannot be proven inactive; all entries were preserved";
pub(super) const CHANGED: &str =
    "unbound socket namespace changed during recovery; no replacement was adopted";
pub(super) const STAGE_BUSY: &str = "unbound socket namespace has active socket publication";
pub(super) const CERTIFICATE_PRESENT: &str = "unbound recovery requires an uncertified directory";
pub(super) const TARGET_OCCUPIED: &str = "unbound socket namespace quarantine name is occupied";
pub(super) const QUARANTINED: &str = "preserved inactive unbound socket namespace in quarantine";
