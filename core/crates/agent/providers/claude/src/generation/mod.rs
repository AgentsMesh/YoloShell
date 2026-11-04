#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../test_support/generation/adversarial_test_support.rs"]
pub(crate) mod adversarial_test_support;
mod authority_methods;
mod authority_set;
mod binding;
mod binding_transition;
mod binding_transition_evidence;
mod binding_transition_proof;
mod binding_transition_stage;
mod binding_transition_validation;
mod constants;
mod durable_binding_io;
mod durable_publication;
mod foreground_store;
mod hook_conversion;
mod lifecycle_lock;
mod lifecycle_store;
mod managed_shim_exec_publication;
#[path = "mutation.rs"]
mod mutation;
pub mod native;
mod observed_unmanaged_publication;
mod observed_unmanaged_publication_validation;
mod orphan_store;
mod process_exit;
mod publication_store;
#[path = "reboot_validation.rs"]
mod reboot_validation;
mod record;
mod recovery_admission;
mod recovery_store;
mod store;
mod store_state;
mod termination;
mod transition;
mod transition_mutation;
mod types;
mod validation;

pub use authority_set::ClaudeInvocationAuthorities;
#[cfg(any(test, feature = "runtime-component-tests"))]
pub const AUTHORITY_SCHEMA_VERSION: u32 = constants::AUTHORITY_SCHEMA_VERSION;
pub use hook_conversion::from_hook_event;
pub use hook_conversion::invocation_mode as from_hook_invocation_mode;
pub use observed_unmanaged_publication::{HookPublicationAdmission, HookPublicationFence};
pub use process_exit::ProcessExitDisposition;
pub use reboot_validation::source_before_reboot_migration;
pub use store::ClaudeGenerationStore;
pub use termination::{SessionEndReason, SessionEndTermination};
pub use types::AuthorityProcessIdentity;
pub use types::{AppliedGenerationEvent, GenerationApplyDisposition};
pub use types::{
    AuthorityAuthentication, AuthorityRuntimeIdentity, ClaudeGenerationAuthority, GenerationBegin,
    GenerationEvent, GenerationRecoveryEvent, InvocationMode, SessionStartSource,
};

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../test_support/generation_integration.rs"]
#[cfg(test)]
mod integration_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/generation/mod.rs"]
mod tests;
