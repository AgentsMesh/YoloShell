mod acquire;
mod authority;
pub mod constants;
mod errors;
mod lifecycle;
mod types;

pub use authority::LeaseAuthority;
pub use types::{
    LeaseEventPayload, LeaseReason, LeaseRequest, LeaseTransition, LeaseTransitionKind,
};

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/tests.rs"]
mod tests;
