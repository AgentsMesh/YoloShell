//! Provider metadata and capability contracts. No provider implementation or
//! runtime construction lives here.
mod capability;
mod constants;
mod descriptor;
mod interrupt;
mod launch;
mod policies;
mod semantics;
mod submit;

pub use capability::{ProviderCapabilities, ProviderTier};
pub use constants::{
    MAX_PROVIDER_DISPLAY_NAME_BYTES, MAX_PROVIDER_ICON_BYTES, MAX_PROVIDER_MANIFEST_ID_BYTES,
    MAX_PROVIDER_TOKEN_BYTES, PROVIDER_DESCRIPTOR_SCHEMA_VERSION, fields,
};
pub use descriptor::{ProviderDescriptor, ProviderDescriptorError};
pub use interrupt::InterruptDelivery;
pub use launch::LaunchPolicies;
pub use policies::ProjectionPolicies;
pub use semantics::{AuthorityKind, InteractionSemantics, LifecycleSemantics, ProjectionPolicy};
pub use submit::PromptSubmit;

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod tests;
