//! Provider-neutral hook-generation authority. The provider supplies ingress
//! evidence and canonical hook events; this crate owns durable replay,
//! reducer state, binding projection and compare-exchange publication.
mod apply;
#[cfg(test)]
mod constants;
mod evidence;
mod origin;
mod projection;
mod record;
mod store;

pub use evidence::GenerationEvidence;
pub use projection::{ExitRule, ProcessRule, ProjectionRules, StopRule, UnknownRule, project};
pub use record::{GENERATION_AUTHORITY_FILE, GenerationScope, HookGenerationRecord};
pub use store::{GenerationRequest, HookGenerationStore};

pub use yolo_hooks_runtime::{
    EventKind, HookMetadata, HookProviderKind, NotificationKind, ProviderVersion,
    SessionEndReason, SessionStartSource,
};

pub const FIRST_SEQUENCE: u64 = yolo_hooks_runtime::FIRST_SEQUENCE;

#[cfg(test)]
#[path = "../unit_tests/fixture.rs"]
mod fixture;

#[cfg(test)]
#[path = "../unit_tests/authority_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../unit_tests/replay_tests.rs"]
mod replay_tests;

#[cfg(test)]
#[path = "../unit_tests/resume_origin_tests.rs"]
mod resume_origin_tests;
