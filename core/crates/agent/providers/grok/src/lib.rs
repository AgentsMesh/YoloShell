pub use yolo_agent_adapter_contract as adapter;
pub use yolo_agent_contract::*;
pub use yolo_agent_core::{constants as core_constants, matching};

#[path = "mod.rs"]
pub mod grok;

pub mod agent {
    pub use yolo_agent_adapter_contract as adapter;
    pub use yolo_agent_adapter_contract::{
        AgentAdapter, DetectionContext, DetectionEnvironment, EvidenceDiscoveryPolicy,
        SessionGenerationPolicy,
    };
    pub use yolo_agent_contract::*;
    pub use yolo_agent_core::{constants, matching};
}

pub use grok::*;

#[cfg(test)]
#[path = "../unit_tests/tests.rs"]
mod tests;
