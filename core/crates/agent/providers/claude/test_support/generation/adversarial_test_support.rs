#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[path = "adversarial_test_support/boot.rs"]
mod boot;
#[path = "adversarial_test_support/native.rs"]
mod native;
#[path = "adversarial_test_support/publication_snapshot.rs"]
mod publication_snapshot;
#[path = "adversarial_test_support/state.rs"]
mod state;
#[path = "adversarial_test_support/state_binding.rs"]
mod state_binding;
#[path = "adversarial_test_support/state_validation.rs"]
mod state_validation;

pub use native::{
    persist_native_durable_state, persist_native_previous_boot, persist_native_revoked,
};

#[cfg(any(test, feature = "adversarial-test-hooks"))]
pub(crate) use boot::{persist_previous_boot, rewrite_boot, rewrite_set_boot};
pub(crate) use publication_snapshot::publication_snapshot;
pub(crate) use state::persist_durable_state;

#[cfg(test)]
pub(crate) use state::{
    PersistenceBoundary, persist_durable_state_with_boundary, prepare_durable_state_for_test,
};
