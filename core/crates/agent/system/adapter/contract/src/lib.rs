#[path = "adapter.rs"]
mod adapter;
#[path = "constants.rs"]
pub mod constants;
#[path = "context.rs"]
mod context;
#[path = "validation.rs"]
pub mod validation;

pub use adapter::{AgentAdapter, EvidenceDiscoveryPolicy, SessionGenerationPolicy};
pub use context::{DetectionContext, DetectionEnvironment, LaunchEnvironment};
pub use validation::validate_launch_environment;
