//! One child-bound, live Hub-owner authorization for a detached broker handoff.
mod constants;
mod frame;
mod issuer;
mod receiver;
mod scope;
mod signature;
mod wire;

pub use constants::{AUTHORITY_STDIO_ARGUMENT, AUTHORITY_STDIO_NAME, START_HUB_COMMAND};
pub use issuer::{LaunchIssuer, issue};
pub use receiver::HubStartAuthorityGuard;
pub use scope::HubBrokerLaunchScope;

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod tests;
