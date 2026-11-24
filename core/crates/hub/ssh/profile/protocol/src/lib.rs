//! Public profile registry DTOs. Validation is lexical and performs no filesystem access.
mod change;
mod configuration;
mod constants;
mod launch;
mod profile;
mod state;
pub use change::SshProfileChange;
pub use configuration::SshProfileConfiguration;
pub use constants::{
    FIRST_SSH_PROFILE_REVISION, MAX_BROKER_STATE_PATH_BYTES, MAX_SSH_PROFILES,
    SSH_PROFILE_REVISION_STEP, SSH_PROFILE_SCHEMA_VERSION,
};
pub use launch::SshBrokerLaunch;
pub use profile::SshBrokerProfile;
pub use state::SshProfileState;
pub use yolo_ssh_profile_protocol::SshProfileRef;
