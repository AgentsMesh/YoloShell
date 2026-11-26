pub const TUNNEL_COUNT: usize = 2;
pub const SOLE_RUNTIME_OWNER: usize = 1;
pub const PROFILE_RECORD: &str = "profiles.json";
pub const SAVED_RECORD: &str = "saved-profile-record";
pub const SAVED_ROOT: &str = "saved-profile-root";
pub const PEER_STATE_DIRECTORY: &str = "lifecycle-peer";
pub const PEER_HOST_DIRECTORY: &str = "lifecycle-peer-host";
pub const PENDING_RESPONSE: &str =
    "registry read unexpectedly yielded before rejecting lost authority";
pub const CLOSURE_OBSERVATION: std::time::Duration = std::time::Duration::from_secs(2);
