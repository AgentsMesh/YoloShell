//! Public-client grants are independent of federation peer enrollment.
mod configuration;
mod constants;
mod error;
mod grant;
mod identity;
mod origin;

pub use configuration::AccessConfiguration;
pub use constants::{
    ACCESS_REVISION_STEP,
    ACCESS_SCHEMA_VERSION, ED25519_PUBLIC_KEY_BYTES, FIRST_ACCESS_REVISION, MAX_CLIENT_ORIGINS,
    MAX_CLIENTS,
};
pub use error::AccessError;
pub use grant::ClientGrant;
pub use identity::ClientIdentity;
pub use origin::OriginAccess;
mod change;
pub use change::ClientAccessChange;
