use std::time::Duration;
pub(crate) const PROFILE_RECHECK: Duration = Duration::from_secs(1);
pub(crate) const REGISTRY_CHANGED: &str = "SSH profile binding changed during peer connection";
pub(crate) const POISONED: &str = "Hub SSH profile registry lock is poisoned";
pub(crate) const TUNNEL_CAPACITY: &str = "Hub SSH tunnel capacity is exhausted";
pub(crate) const LAUNCH_BUSY: &str = "SSH profile has an active startup or provisioning operation";
pub(crate) const LAUNCH_UNAVAILABLE: &str = "this Hub cannot start an SSH broker";
pub(crate) const LAUNCH_DEADLINE: &str = "SSH broker startup deadline expired";
pub(crate) const CONNECT_FAILED: &str =
    "SSH profile connection failed; operation outcome is unknown";
