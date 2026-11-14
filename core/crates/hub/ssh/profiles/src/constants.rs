pub const SSH_PROFILES_DIRECTORY: &str = "ssh-profiles";
pub(crate) const RECORD_SCHEMA: u16 = 1;
pub(crate) const PROFILE_FILE: &str = "profiles.json";
pub(crate) const LOCK_FILE: &str = "profiles.lock";
pub(crate) const LOCK_BYTES: u64 = 4096;
pub(crate) const NAMESPACE: &str = "hub-ssh-profiles";
pub(crate) const BUSY: &str = "Hub SSH profile store is already open";
pub(crate) const IDENTITY_CHANGED: &str = "Hub SSH profile store identity changed";
pub(crate) const SEED_CONFLICT: &str = "Hub SSH profile seed differs from committed profiles";
pub(crate) const UNKNOWN_PROFILE: &str = "SSH profile is absent or its authority revision changed";
pub(crate) const LOST: &str =
    "Hub SSH profile store lost its retained authority; reopen is required";
pub(crate) const REVISION_CONFLICT: &str =
    "Hub SSH profile revision conflicts with the committed state";
pub(crate) const REVISION_EXHAUSTED: &str = "Hub SSH profile revision is exhausted";
